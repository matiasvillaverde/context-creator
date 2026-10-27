use super::{
    digest,
    index::{relative, Snapshot},
    recipes::Recipe,
    types::*,
};
use anyhow::{bail, Result};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
};

pub(crate) struct Candidate {
    pub snippet: Snippet,
    pub signature: Option<String>,
    pub score: i64,
    pub primary: bool,
    pub bytes: (usize, usize),
}

pub(crate) struct Selection {
    pub candidates: Vec<Candidate>,
    pub relations: Vec<Relation>,
    pub gaps: Vec<Gap>,
}

pub(crate) fn words(text: &str) -> BTreeSet<String> {
    let mut expanded = String::new();
    let mut previous = ' ';
    for ch in text.chars() {
        if ch.is_uppercase() && previous.is_lowercase() {
            expanded.push(' ');
        }
        expanded.extend(ch.to_lowercase());
        previous = ch;
    }
    expanded
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| word.len() >= 3)
        .filter(|word| {
            ![
                "the", "and", "for", "with", "from", "this", "that", "fix", "find", "explain",
                "feature", "debug", "failure", "review", "change", "code", "how", "does", "early",
            ]
            .contains(word)
        })
        .map(|word| match word {
            "expiration" | "expiry" | "expired" | "expire" | "expires" | "expiring" => {
                "expir".into()
            }
            "authentication" | "authenticate" | "authenticator" => "auth".into(),
            _ => word.trim_end_matches('s').to_string(),
        })
        .collect()
}

fn reason(
    kind: &str,
    detail: String,
    confidence: Confidence,
    source: Option<Location>,
) -> SelectionReason {
    SelectionReason {
        kind: kind.into(),
        detail,
        confidence,
        source,
    }
}

pub(crate) fn select(
    snapshot: &Snapshot,
    request: &ContextRequest,
    recipe: &Recipe,
    changes: &[Change],
) -> Result<Selection> {
    let terms = words(&request.task);
    let mut ranks: BTreeMap<String, (i64, Vec<SelectionReason>)> = BTreeMap::new();
    let mut seeds = BTreeSet::new();
    for seed in &request.seeds {
        let name = relative(seed)?;
        let matches = snapshot
            .sources
            .keys()
            .filter(|path| **path == name || path.starts_with(&(name.clone() + "/")))
            .cloned()
            .collect::<Vec<_>>();
        if matches.is_empty() {
            bail!("Seed is missing, ignored, binary, hidden, or outside scope: {name}");
        }
        for path in matches {
            seeds.insert(path.clone());
            ranks.insert(
                path,
                (
                    1200,
                    vec![reason(
                        "seed",
                        "Explicit agent seed".into(),
                        Confidence::Resolved,
                        None,
                    )],
                ),
            );
        }
    }
    let filters = request
        .include_patterns
        .iter()
        .chain(recipe.include_patterns.iter())
        .map(|pattern| glob::Pattern::new(pattern))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let preferences = recipe
        .priority_patterns
        .iter()
        .map(|pattern| glob::Pattern::new(pattern).map(|compiled| (pattern, compiled)))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut lexical = Vec::new();
    for (path, source) in &snapshot.sources {
        if (!recipe.include_tests && is_test(path)
            || !recipe.include_config && is_config(Path::new(path)))
            && !seeds.contains(path)
        {
            continue;
        }
        if !filters.is_empty()
            && !filters.iter().any(|pattern| pattern.matches(path))
            && !seeds.contains(path)
        {
            continue;
        }
        let path_words = words(path);
        let text_words = words(&source.text);
        let score = terms.intersection(&path_words).count() * 50
            + terms.intersection(&text_words).count() * 20;
        let preference = preferences
            .iter()
            .find(|(_, pattern)| pattern.matches(path));
        if score > 0 || preference.is_some() {
            lexical.push((
                path.clone(),
                score as i64 + if preference.is_some() { 600 } else { 0 },
                preference.map(|(pattern, _)| (*pattern).clone()),
            ));
        }
    }
    lexical.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (path, score, preference) in lexical.into_iter().take(12) {
        let entry = ranks.entry(path.clone()).or_insert((0, vec![]));
        entry.0 += score;
        if let Some(pattern) = preference {
            entry.1.push(reason(
                "recipe_priority",
                format!("Recipe path preference: {pattern}"),
                Confidence::Inferred,
                None,
            ));
        }
        if !terms.is_disjoint(&words(&path))
            || !terms.is_disjoint(&words(&snapshot.sources[&path].text))
        {
            entry.1.push(reason(
                "task_match",
                "Task terms match source identifiers or path".into(),
                Confidence::Inferred,
                None,
            ));
        }
    }
    let identifiers = if changes
        .iter()
        .any(|change| !change.removed_symbols.is_empty())
    {
        snapshot
            .sources
            .iter()
            .map(|(path, source)| {
                let names = source
                    .text
                    .split(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '$'))
                    .filter(|name| !name.is_empty())
                    .map(str::to_lowercase)
                    .collect::<BTreeSet<_>>();
                (path.as_str(), names)
            })
            .collect::<BTreeMap<_, _>>()
    } else {
        BTreeMap::new()
    };
    for change in changes {
        if snapshot.sources.contains_key(&change.path) {
            ranks.insert(
                change.path.clone(),
                (
                    1500,
                    vec![reason(
                        "changed",
                        format!("{} relative to Git baseline", change.status),
                        Confidence::Resolved,
                        None,
                    )],
                ),
            );
        }
        for path in snapshot.sources.keys() {
            if change.removed_symbols.iter().any(|symbol| {
                identifiers
                    .get(path.as_str())
                    .is_some_and(|names| names.contains(symbol.to_lowercase().as_str()))
            }) && *path != change.path
            {
                let entry = ranks.entry(path.clone()).or_insert((0, vec![]));
                entry.0 = entry.0.max(400);
                entry.1.push(reason(
                    "removed_symbol_reference",
                    "Source mentions a removed symbol; binding is unverified".into(),
                    Confidence::Inferred,
                    Some(Location {
                        path: change.path.clone(),
                        line: 1,
                    }),
                ));
            }
        }
    }
    let initial = ranks.clone();
    let mut relations = Vec::new();
    let mut import_limit =
        recipe.depth > 0 && expand_import_tree(snapshot, request, &mut ranks, &mut relations)?;
    let primary_paths = ranks.keys().cloned().collect::<BTreeSet<_>>();
    let mut neighborhood = initial.keys().cloned().collect::<BTreeSet<_>>();
    for depth in 0..recipe.depth {
        let frontier = ranks
            .iter()
            .filter(|(path, _)| neighborhood.contains(*path))
            .map(|(path, rank)| (path.clone(), rank.clone()))
            .collect::<BTreeMap<_, _>>();
        for (path, (score, _)) in &frontier {
            if *score < 100 {
                continue;
            }
            let Some(source) = snapshot.sources.get(path) else {
                continue;
            };
            for (kind, links, confidence) in [
                ("dependency", &source.info.imports, Confidence::Inferred),
                ("dependent", &source.info.imported_by, Confidence::Inferred),
            ] {
                let mut links = links.iter().collect::<Vec<_>>();
                links.sort();
                for target in links {
                    let source_line = if kind == "dependency" {
                        source
                            .resolved_imports
                            .get(target)
                            .map(|edge| edge.line)
                            .unwrap_or(1)
                    } else {
                        1
                    };
                    let confidence = if kind == "dependency" {
                        source
                            .resolved_imports
                            .get(target)
                            .map(|edge| edge.confidence)
                            .unwrap_or(confidence)
                    } else {
                        confidence
                    };
                    let Ok(relative_path) = target.strip_prefix(&request.root) else {
                        continue;
                    };
                    let Ok(target) = relative(relative_path) else {
                        continue;
                    };
                    if !snapshot.sources.contains_key(&target) || target == *path {
                        continue;
                    }
                    let relation = Relation {
                        from: Location {
                            path: path.clone(),
                            line: source_line,
                        },
                        to: Location {
                            path: target.clone(),
                            line: 1,
                        },
                        kind: kind.into(),
                        confidence,
                        evidence: if kind == "dependency" {
                            dependency_evidence(confidence)
                        } else {
                            "Static importing or calling client; runtime dispatch is unverified"
                        }
                        .into(),
                    };
                    if relations.len() < 1024 && !relations.contains(&relation) {
                        relations.push(relation);
                    }
                    if ranks.len() >= 256 && !ranks.contains_key(&target) {
                        continue;
                    }
                    neighborhood.insert(target.clone());
                    let lexical_anchor = initial.get(&target).is_some_and(|(_, reasons)| {
                        reasons.iter().all(|reason| reason.kind == "task_match")
                    });
                    let entry = ranks.entry(target).or_insert((0, vec![]));
                    entry.0 = entry
                        .0
                        .max(score.saturating_sub(150 + depth as i64 * 50).max(100));
                    if !lexical_anchor && entry.1.len() < 4 {
                        let why = reason(
                            kind,
                            "Related through the static project graph".into(),
                            confidence,
                            Some(Location {
                                path: path.clone(),
                                line: source_line,
                            }),
                        );
                        if !entry.1.contains(&why) {
                            entry.1.push(why);
                        }
                    }
                }
            }
        }
        if recipe.follow_contracts {
            for relation in &snapshot.contract_relations {
                for (from, to) in [
                    (&relation.from, &relation.to),
                    (&relation.to, &relation.from),
                ] {
                    if let Some((score, _)) = frontier.get(&from.path) {
                        if ranks.len() >= 256 && !ranks.contains_key(&to.path) {
                            continue;
                        }
                        neighborhood.insert(to.path.clone());
                        let entry = ranks.entry(to.path.clone()).or_insert((0, vec![]));
                        entry.0 = entry.0.max(score.saturating_sub(100).max(100));
                        let why = reason(
                            &relation.kind,
                            relation.evidence.clone(),
                            relation.confidence,
                            Some(from.clone()),
                        );
                        if entry.1.len() < 4 && !entry.1.contains(&why) {
                            entry.1.push(why);
                        }
                        if relations.len() < 1024 && !relations.contains(relation) {
                            relations.push(relation.clone());
                        }
                    }
                }
            }
        }
        if ranks.len() > 256 {
            break;
        }
    }
    if recipe.include_tests || recipe.include_config {
        let zig_context = initial.keys().any(|path| path.ends_with(".zig"));
        let focus_words = initial
            .keys()
            .flat_map(|path| words(path))
            .chain(terms.clone())
            .filter(|word| {
                !["src", "test", "config", "main", "lib", "service"].contains(&word.as_str())
            })
            .collect::<BTreeSet<_>>();
        for (path, source) in &snapshot.sources {
            let test_file = is_test(path);
            let config = is_config(Path::new(path));
            if !(test_file && recipe.include_tests || config && recipe.include_config) {
                continue;
            }
            let score = words(path).intersection(&focus_words).count()
                + words(&source.text).intersection(&focus_words).count();
            if score > 0 || zig_context && matches!(path.as_str(), "build.zig" | "build.zig.zon") {
                let entry = ranks.entry(path.clone()).or_insert((0, vec![]));
                entry.0 = entry.0.max(300 + score as i64 * 10);
                entry.1.push(reason(
                    if test_file {
                        "related_test"
                    } else {
                        "related_configuration"
                    },
                    "Path or content overlaps task and selected source".into(),
                    Confidence::Inferred,
                    None,
                ));
            }
        }
    }
    // The recipe bounds surrounding callers/contracts. Follow the forward import
    // closure separately so a seed's dependencies do not disappear after 2-3 hops.
    import_limit |=
        recipe.depth > 0 && expand_import_tree(snapshot, request, &mut ranks, &mut relations)?;
    let mut ranked = ranks.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        primary_paths
            .contains(&b.0)
            .cmp(&primary_paths.contains(&a.0))
            .then(b.1 .0.cmp(&a.1 .0))
            .then(a.0.cmp(&b.0))
    });
    let mut candidates = Vec::new();
    let mut gaps = Vec::new();
    if import_limit || ranked.len() >= 256 || relations.len() >= 1024 {
        gaps.push(Gap {
            kind: "graph_limit".into(),
            path: None,
            line: None,
            detail: "Task expansion reached its 256-file or 1024-relationship limit".into(),
        });
    }
    if ranked.len() > 128 {
        gaps.push(Gap {
            kind: "candidate_limit".into(),
            path: None,
            line: None,
            detail: format!("{} ranked files were not expanded into snippets because of the 128-file candidate limit", ranked.len() - 128),
        });
    }
    if ranked.is_empty() {
        gaps.push(Gap {
            kind: "no_matches".into(),
            path: None,
            line: None,
            detail: "No local task matches; provide seeds or more specific terms".into(),
        });
    }
    for (path, (score, reasons)) in ranked.into_iter().take(128) {
        if (!recipe.include_tests && is_test(&path)
            || !recipe.include_config && is_config(Path::new(&path)))
            && !seeds.contains(&path)
            && !changes.iter().any(|change| change.path == path)
        {
            gaps.push(Gap {
                kind: "recipe_exclusion".into(),
                path: Some(path),
                line: None,
                detail: "File was found but omitted by the recipe's test/configuration policy"
                    .into(),
            });
            continue;
        }
        let source = &snapshot.sources[&path];
        if recipe.depth == 0 && !source.resolved_imports.is_empty() {
            gaps.push(Gap {
                kind: "dependency_traversal_disabled".into(),
                path: Some(path.clone()),
                line: None,
                detail: format!(
                    "{} local imports were not followed because this recipe sets depth to zero",
                    source.resolved_imports.len()
                ),
            });
        }
        let filename_match = !terms.is_disjoint(&words(&path));
        let declaration_match = source.declarations.iter().any(|declaration| {
            !terms.is_disjoint(&words(
                &source.text[declaration.start_byte..declaration.end_byte],
            ))
        });
        if source.parse_error {
            gaps.push(Gap {
                kind: "parse_error".into(),
                path: Some(path.clone()),
                line: None,
                detail: "Malformed source: declarations and relationships may be incomplete".into(),
            });
        }
        if source.declarations.is_empty() {
            if !source.text.is_empty() {
                candidates.push(candidate(
                    &path,
                    None,
                    "file",
                    1,
                    source.text.lines().count().max(1),
                    source.text.to_string(),
                    None,
                    score,
                    reasons.clone(),
                    (0, source.text.len()),
                ));
            }
        } else {
            let mut occurrences = BTreeMap::new();
            if source.declarations.len() > 256 {
                gaps.push(Gap { kind: "declaration_limit".into(), path: Some(path.clone()), line: None, detail: "Only the first 256 declarations in this file are considered; narrow the source".into() });
            }
            for declaration in source.declarations.iter().take(256) {
                let name = &declaration.qualified_name;
                let occurrence = occurrences.entry(name).or_insert(0usize);
                *occurrence += 1;
                let content = source.text[declaration.start_byte..declaration.end_byte].to_string();
                let symbol_score =
                    terms.intersection(&words(&declaration.symbol.name)).count() as i64 * 100;
                let text_score = terms.intersection(&words(&content)).count() as i64 * 15;
                if reasons.iter().all(|reason| reason.kind == "task_match")
                    && symbol_score == 0
                    && text_score == 0
                    && !(filename_match && !declaration_match)
                {
                    continue;
                }
                let changed = changes
                    .iter()
                    .find(|change| change.path == path)
                    .is_some_and(|change| {
                        change.ranges.iter().any(|range| {
                            range.start <= declaration.end_line
                                && range.end >= declaration.symbol.line
                        })
                    });
                let mut item = candidate(
                    &path,
                    Some(format!("{name}#{occurrence}")),
                    &declaration.symbol.kind,
                    declaration.start_line,
                    declaration.end_line,
                    content,
                    Some(declaration.signature.clone()),
                    score + symbol_score + text_score + if changed { 500 } else { 0 },
                    reasons.clone(),
                    (declaration.start_byte, declaration.end_byte),
                );
                item.snippet.symbol = Some(declaration.symbol.name.clone());
                if filename_match && !declaration_match {
                    item.snippet.reasons.push(reason(
                        "filename_match",
                        "File path matches the task; no declaration repeats its terms".into(),
                        Confidence::Inferred,
                        None,
                    ));
                }
                candidates.push(item);
            }
            let mut spans = source
                .declarations
                .iter()
                .map(|declaration| (declaration.start_byte, declaration.end_byte))
                .collect::<Vec<_>>();
            spans.sort_unstable();
            let mut offset = 0;
            let mut section = 0;
            for (start, end) in spans
                .into_iter()
                .chain(std::iter::once((source.text.len(), source.text.len())))
            {
                if start > offset && !source.text[offset..start].trim().is_empty() {
                    section += 1;
                    let prefix = &source.text[..offset];
                    let content = &source.text[offset..start];
                    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
                    let mut item = candidate(
                        &path,
                        Some(format!("section#{section}")),
                        "file_context",
                        line,
                        line + content.lines().count().saturating_sub(1),
                        content.into(),
                        None,
                        score - 20,
                        reasons.clone(),
                        (offset, start),
                    );
                    item.snippet.symbol = None;
                    candidates.push(item);
                }
                offset = offset.max(end);
            }
        }
        for reference in &source.info.type_references {
            if let Some(package) = &reference.external_package {
                gaps.push(Gap {
                    kind: "external_type".into(),
                    path: Some(path.clone()),
                    line: Some(reference.line),
                    detail: format!(
                        "Type {} uses {package}; external source is not included",
                        reference.name
                    ),
                });
            }
            if reference.definition_path.is_none() && !reference.is_external {
                gaps.push(Gap {
                    kind: "unresolved_type".into(),
                    path: Some(path.clone()),
                    line: Some(reference.line),
                    detail: format!("Type {} has no resolved local definition", reference.name),
                });
            }
        }
        for (module, line) in &source.unresolved_imports {
            gaps.push(Gap {
                kind: "unresolved_import".into(),
                path: Some(path.clone()),
                line: Some(*line),
                detail: format!("Import {module} has no unique visible local definition"),
            });
        }
        for (module, line) in &source.external_imports {
            gaps.push(Gap {
                kind: "external_import".into(),
                path: Some(path.clone()),
                line: Some(*line),
                detail: format!("Import {module} requires external or build-provided source; its implementation is not included"),
            });
        }
    }
    if recipe.follow_contracts {
        gaps.extend(
            snapshot
                .contract_gaps
                .iter()
                .filter(|gap| {
                    gap.path.as_ref().is_some_and(|path| {
                        candidates.iter().any(|item| item.snippet.path == *path)
                    })
                })
                .cloned(),
        );
    }
    gaps.push(Gap { kind: "static_analysis".into(), path: None, line: None, detail: "Selection uses lexical matches and static relationships; dynamic imports, dispatch, service origins, and compiler classpaths may be unresolved".into() });
    for candidate in &mut candidates {
        candidate.primary = primary_paths.contains(&candidate.snippet.path);
    }
    candidates.sort_by(|a, b| {
        b.primary
            .cmp(&a.primary)
            .then(b.score.cmp(&a.score))
            .then(a.snippet.path.cmp(&b.snippet.path))
            .then(a.snippet.start_line.cmp(&b.snippet.start_line))
    });
    relations.sort_by(|left, right| {
        (
            &left.from.path,
            left.from.line,
            &left.to.path,
            left.to.line,
            &left.kind,
        )
            .cmp(&(
                &right.from.path,
                right.from.line,
                &right.to.path,
                right.to.line,
                &right.kind,
            ))
    });
    if candidates.len() > 512 {
        gaps.push(Gap {
            kind: "candidate_limit".into(),
            path: None,
            line: None,
            detail: format!(
                "{} lower-ranked snippets excluded before fitting; narrow task or seeds",
                candidates.len() - 512
            ),
        });
        candidates.truncate(512);
    }
    Ok(Selection {
        candidates,
        relations,
        gaps,
    })
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    path: &str,
    identity: Option<String>,
    kind: &str,
    start: usize,
    end: usize,
    content: String,
    signature: Option<String>,
    score: i64,
    reasons: Vec<SelectionReason>,
    bytes: (usize, usize),
) -> Candidate {
    Candidate {
        snippet: Snippet {
            id: digest(format!("{path}\0{kind}\0{}", identity.as_deref().unwrap_or("")).as_bytes()),
            path: path.into(),
            symbol: identity,
            kind: kind.into(),
            start_line: start,
            end_line: end,
            representation: Representation::Full,
            content_hash: digest(content.as_bytes()),
            content: Some(content),
            reasons: reasons.into_iter().take(4).collect(),
        },
        signature,
        score,
        primary: false,
        bytes,
    }
}

pub(crate) fn is_test(path: &str) -> bool {
    path.split('/').any(|part| {
        part == "tests"
            || part == "test"
            || part.starts_with("test_")
            || part.contains(".test.")
            || part.contains(".spec.")
            || part.ends_with("_test.rs")
            || part.ends_with("_test.go")
    })
}
fn is_config(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("build.zig" | "build.zig.zon")
    ) || path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ["toml", "json", "yaml", "yml", "ini", "cfg"].contains(&ext))
}

fn dependency_evidence(confidence: Confidence) -> &'static str {
    if confidence == Confidence::Resolved {
        "Static local file or module dependency"
    } else {
        "Local dependency candidate; runtime, namespace, or search-path binding is unverified"
    }
}

fn expand_import_tree(
    snapshot: &Snapshot,
    request: &ContextRequest,
    ranks: &mut BTreeMap<String, (i64, Vec<SelectionReason>)>,
    relations: &mut Vec<Relation>,
) -> Result<bool> {
    let mut pending = ranks.keys().cloned().collect::<VecDeque<_>>();
    let mut queued = ranks.keys().cloned().collect::<BTreeSet<_>>();
    let mut edges = relations
        .iter()
        .enumerate()
        .filter(|(_, relation)| relation.kind == "dependency")
        .map(|(index, relation)| {
            (
                (relation.from.path.clone(), relation.to.path.clone()),
                index,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut limited = false;
    while let Some(path) = pending.pop_front() {
        let Some(source) = snapshot.sources.get(&path) else {
            continue;
        };
        let score = ranks.get(&path).map(|rank| rank.0).unwrap_or(100);
        for (target, import) in &source.resolved_imports {
            let target = relative(target.strip_prefix(&request.root)?)?;
            if !snapshot.sources.contains_key(&target) {
                continue;
            }
            let relation = Relation {
                from: Location {
                    path: path.clone(),
                    line: import.line,
                },
                to: Location {
                    path: target.clone(),
                    line: 1,
                },
                kind: "dependency".into(),
                confidence: import.confidence,
                evidence: dependency_evidence(import.confidence).into(),
            };
            let key = (path.clone(), target.clone());
            if let Some(index) = edges.get(&key) {
                relations[*index] = relation;
            } else if relations.len() < 1024 {
                edges.insert(key, relations.len());
                relations.push(relation);
            } else {
                limited = true;
            }
            if ranks.len() >= 256 && !ranks.contains_key(&target) {
                limited = true;
                continue;
            }
            let already_selected = ranks.contains_key(&target);
            let entry = ranks.entry(target.clone()).or_insert((0, vec![]));
            entry.0 = entry.0.max(score.saturating_sub(100).max(100));
            let why = reason(
                "dependency",
                "Reachable through a local import".into(),
                import.confidence,
                Some(Location {
                    path: path.clone(),
                    line: import.line,
                }),
            );
            // Existing lexical anchors keep their declaration-level relevance filter.
            if !already_selected && entry.1.len() < 4 && !entry.1.contains(&why) {
                entry.1.push(why);
            }
            if queued.insert(target.clone()) {
                pending.push_back(target);
            }
        }
    }
    Ok(limited)
}
