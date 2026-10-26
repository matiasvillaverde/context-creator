use super::{digest, types::*};
use crate::{
    cli::Config,
    core::{
        cache::FileCache,
        project_analyzer::ProjectAnalysis,
        semantic::symbols::{extract_declarations_with_diagnostics, SourceDeclaration},
        walker::{walk_directory, FileInfo, WalkOptions},
    },
};
use anyhow::{bail, Context, Result};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Component, Path},
    sync::Arc,
};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILES: usize = 5000;

#[derive(Debug, Clone)]
pub(crate) struct Source {
    pub info: FileInfo,
    pub text: Arc<str>,
    pub hash: String,
    pub declarations: Arc<Vec<SourceDeclaration>>,
    pub parse_error: bool,
    pub raw_imports: Arc<Vec<crate::core::semantic::analyzer::Import>>,
    pub resolved_imports: BTreeMap<std::path::PathBuf, usize>,
    pub unresolved_imports: Vec<(String, usize)>,
}

pub(crate) struct Snapshot {
    pub revision: String,
    pub sources: BTreeMap<String, Source>,
    pub contract_relations: Vec<Relation>,
    pub contract_gaps: Vec<Gap>,
}

pub(crate) fn relative(path: &Path) -> Result<String> {
    let mut names = Vec::new();
    for part in path.components() {
        let Component::Normal(name) = part else {
            bail!("Paths must be relative without traversal");
        };
        let name = name.to_str().context("Source paths must be UTF-8")?;
        if name.starts_with('.') || name.contains(['\n', '\r', '\0']) {
            bail!("Hidden or control-character paths are unavailable");
        }
        names.push(name);
    }
    if names.is_empty() {
        bail!("A source path cannot be empty");
    }
    Ok(names.join("/"))
}

pub(crate) fn snapshot(
    root: &Path,
    config: &Config,
    previous: Option<&Snapshot>,
) -> Result<(Arc<Snapshot>, IndexStats)> {
    let mut options = WalkOptions::from_config(config)?;
    // Include filters select context; they never conceal dependency candidates from analysis.
    options.include_patterns.clear();
    options.max_file_size = Some(MAX_FILE_BYTES as usize);
    options.filter_binary_files = true;
    let mut files = walk_directory(root, options)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if files.len() > MAX_FILES {
        bail!("Agent context is limited to {MAX_FILES} files; narrow the repository root or ignore patterns");
    }
    let cache = Arc::new(FileCache::with_secret_policy(config.get_secret_policy()));
    let mut sources = BTreeMap::new();
    let mut stats = IndexStats::default();
    let mut revision = Vec::new();
    let mut valid_files = Vec::new();
    for file in files {
        let name = relative(&file.relative_path)?;
        // Validate every component before reading; explicit seeds cannot reopen excluded paths.
        let mut target = root.to_path_buf();
        for part in file.relative_path.components() {
            target.push(part);
            if std::fs::symlink_metadata(&target)?.file_type().is_symlink() {
                bail!("Symlinks are unavailable for agent context");
            }
        }
        let canonical = target.canonicalize()?;
        if !canonical.starts_with(root) {
            bail!("Source path escapes repository root");
        }
        let handle = std::fs::File::open(&target)?;
        if !handle.metadata()?.is_file() {
            bail!("Agent sources must be regular files");
        }
        let mut bytes = Vec::new();
        handle.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            bail!("Source file exceeds the 2 MiB agent limit");
        }
        if bytes.contains(&0) {
            continue;
        }
        let Ok(raw) = std::str::from_utf8(&bytes) else {
            continue;
        };
        stats.source_bytes += bytes.len();
        if stats.source_bytes > MAX_SOURCE_BYTES {
            bail!("Agent source snapshot exceeds 64 MiB; narrow the scope");
        }
        let hash = digest(&bytes);
        revision.extend_from_slice(name.as_bytes());
        revision.push(0);
        revision.extend_from_slice(hash.as_bytes());
        revision.push(0);
        let old = previous
            .and_then(|snapshot| snapshot.sources.get(&name))
            .filter(|source| source.hash == hash);
        let mut source = if let Some(old) = old {
            old.clone()
        } else {
            let text: Arc<str> =
                crate::core::secrets::apply_policy(&target, raw, config.get_secret_policy())?
                    .into();
            let (declarations, parse_error) =
                extract_declarations_with_diagnostics(&target, &text)?;
            let raw_imports =
                if let Some(analyzer) = crate::core::semantic::get_analyzer_for_file(&target)? {
                    let context =
                        crate::core::semantic::SemanticContext::new(target.clone(), root.into(), 3);
                    analyzer
                        .analyze_requested(&target, &text, &context, true, false, false, false)
                        .map(|analysis| analysis.imports)
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
            stats.declarations_reparsed += 1;
            Source {
                info: file.clone(),
                text,
                hash,
                declarations: Arc::new(declarations),
                parse_error,
                raw_imports: Arc::new(raw_imports),
                resolved_imports: BTreeMap::new(),
                unresolved_imports: Vec::new(),
            }
        };
        source.info = file.clone();
        cache.insert_snapshot(canonical, source.text.clone());
        sources.insert(name, source);
        valid_files.push(file);
    }
    stats.files_scanned = sources.len();
    let revision = digest(&revision);
    if let Some(old) = previous.filter(|old| old.revision == revision) {
        stats.analysis_reused = true;
        return Ok((
            Arc::new(Snapshot {
                revision,
                sources: old.sources.clone(),
                contract_relations: old.contract_relations.clone(),
                contract_gaps: old.contract_gaps.clone(),
            }),
            stats,
        ));
    }
    let analysis = ProjectAnalysis::from_files(valid_files, root, config, &cache)?;
    let valid_paths = analysis
        .all_files
        .iter()
        .map(|file| file.path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for file in analysis.all_files {
        let name = relative(&file.relative_path)?;
        if let Some(source) = sources.get_mut(&name) {
            source.info = file;
            source.resolved_imports.clear();
            source.unresolved_imports.clear();
            if let Some(resolver) = crate::core::semantic::get_resolver_for_file(&source.info.path)?
            {
                for import in source.raw_imports.iter() {
                    match resolver.resolve_import(&import.module, &source.info.path, root) {
                        Ok(resolved)
                            if !resolved.is_external && valid_paths.contains(&resolved.path) =>
                        {
                            source.resolved_imports.insert(resolved.path, import.line);
                        }
                        Ok(resolved)
                            if resolved.is_external
                                && !import.is_relative
                                && !import.module.starts_with('.') => {}
                        _ => {
                            source
                                .unresolved_imports
                                .push((import.module.clone(), import.line));
                        }
                    }
                }
            }
        }
    }
    let mut snapshot = Snapshot {
        revision,
        sources,
        contract_relations: Vec::new(),
        contract_gaps: Vec::new(),
    };
    (snapshot.contract_relations, snapshot.contract_gaps) =
        super::contracts::relationships(&snapshot)?;
    Ok((Arc::new(snapshot), stats))
}
