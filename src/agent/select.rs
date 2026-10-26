use super::{
    digest,
    index::{relative, Snapshot},
    recipes::Recipe,
    types::*,
};
use anyhow::{bail, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) struct Candidate {
    pub snippet: Snippet,
    pub signature: Option<String>,
    pub score: i64,
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
        if score > 0 {
            lexical.push((path.clone(), score as i64));
        }
    }
    lexical.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (path, score) in lexical.into_iter().take(12) {
        let entry = ranks.entry(path).or_insert((0, vec![]));
        entry.0 += score;
        entry.1.push(reason(
            "task_match",
            "Task terms match source identifiers or path".into(),
            Confidence::Inferred,
            None,
        ));
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
    for depth in 0..recipe.depth {
        let frontier = ranks.clone();
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
                    let confidence =
                        if kind == "dependency" && source.resolved_imports.contains_key(target) {
                            Confidence::Resolved
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
                            line: 1,
                        },
                        to: Location {
                            path: target.clone(),
                            line: 1,
                        },
                        kind: kind.into(),
                        confidence,
                        evidence: if kind == "dependency" {
                            "Local dependency candidate in the static project graph; binding may be heuristic"
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
                    let entry = ranks.entry(target).or_insert((0, vec![]));
                    entry.0 = entry
                        .0
                        .max(score.saturating_sub(150 + depth as i64 * 50).max(100));
                    if entry.1.len() < 4 {
                        let why = reason(
                            kind,
                            "Related through the static project graph".into(),
                            confidence,
                            Some(Location {
                                path: path.clone(),
                                line: 1,
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
            if score > 0 {
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
    let mut ranked = ranks.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(&b.0)));
    let mut candidates = Vec::new();
    let mut gaps = Vec::new();
    if ranked.len() >= 256 || relations.len() >= 1024 {
        gaps.push(Gap {
            kind: "graph_limit".into(),
            path: None,
            line: None,
            detail: "Task expansion reached its 256-file or 1024-relationship limit".into(),
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
            continue;
        }
        let source = &snapshot.sources[&path];
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
                detail: format!("Import {module} has no visible resolved local definition"),
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
    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
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
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ["toml", "json", "yaml", "yml", "ini", "cfg"].contains(&ext))
}
