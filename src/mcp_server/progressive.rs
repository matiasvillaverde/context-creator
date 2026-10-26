//! Bounded exploration tools shared by both MCP implementations.

use crate::cli::{Config, OutputFormat};
use crate::core::{
    budget::fit_rendered,
    cache::FileCache,
    context_builder::{generate_digest_with_report, ContextOptions},
    secrets::SecretPolicy,
    semantic::path_validator::validate_import_path,
    token::TokenEncoding,
    walker::WalkOptions,
};
use anyhow::{bail, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Component, PathBuf};
use std::sync::Arc;

const MAX_TOKENS: usize = 32_768;
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryMapRequest {
    pub path: PathBuf,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    pub max_tokens: Option<usize>,
    pub tokenizer: Option<TokenEncoding>,
    /// Defaults to redact for this tool. Content scanning is heuristic.
    pub secret_scan: Option<SecretPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryMapResponse {
    pub map: String,
    pub token_count: usize,
    pub file_count: usize,
    pub omitted_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileRequest {
    pub root: PathBuf,
    /// Path relative to root. Hidden files, ignored files, and symlinks are denied.
    pub path: PathBuf,
    /// One-based inclusive starting line (default: 1).
    pub start_line: Option<usize>,
    /// At most 1000 lines (default: 200).
    pub max_lines: Option<usize>,
    pub max_tokens: Option<usize>,
    pub tokenizer: Option<TokenEncoding>,
    pub secret_scan: Option<SecretPolicy>,
    /// Add complete, transitive agent context with this separate JSON token budget (256..=65536).
    /// Omit to read only the requested source page without project analysis.
    pub dependency_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileResponse {
    pub path: PathBuf,
    pub content: String,
    pub start_line: usize,
    /// Inclusive; absent when the requested range is empty.
    pub end_line: Option<usize>,
    pub next_line: Option<usize>,
    pub truncated: bool,
    /// Counts content only, excluding the MCP JSON envelope.
    pub token_count: usize,
    /// Optional dependency context; its token_count bounds its complete compact JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_context: Option<crate::agent::ContextResponse>,
}

fn token_limit(requested: Option<usize>) -> Result<usize> {
    let limit = requested.unwrap_or(4096);
    if !(1..=MAX_TOKENS).contains(&limit) {
        bail!("max_tokens must be between 1 and {MAX_TOKENS}");
    }
    Ok(limit)
}

pub fn repository_map_sync(request: RepositoryMapRequest) -> Result<RepositoryMapResponse> {
    let limit = token_limit(request.max_tokens)?;
    let root = request.path.canonicalize()?;
    if !root.is_dir() {
        bail!("Repository map requires a directory");
    }
    let config = Config {
        paths: Some(vec![root.clone()]),
        output_format: OutputFormat::Map,
        max_tokens: Some(limit),
        tokenizer: request.tokenizer,
        secret_scan: Some(request.secret_scan.unwrap_or(SecretPolicy::Redact)),
        include: Some(request.include_patterns),
        ignore: Some(request.ignore_patterns),
        quiet: true,
        ..Config::default()
    };
    config.validate()?;
    let mut walk_options = WalkOptions::from_config(&config)?;
    walk_options.filter_binary_files = true;
    let cache = Arc::new(FileCache::with_secret_policy(config.get_secret_policy()));
    let context_options = ContextOptions::from_config(&config)?;
    // One walk; cap before parsing/graph work. collect_directory_files applies
    // the same guard for CLI maps and reuses the already walked files.
    let files = crate::collect_directory_files(
        &root,
        walk_options,
        context_options.clone(),
        cache.clone(),
        &config,
    )?;
    let mut selection_options = context_options.clone();
    selection_options.max_tokens = None;
    let files =
        crate::core::prioritizer::prioritize_files(files, &selection_options, cache.clone())?;
    let rendered = generate_digest_with_report(
        files,
        context_options,
        cache,
        OutputFormat::Map,
        &root.to_string_lossy(),
    )?;
    Ok(RepositoryMapResponse {
        map: rendered.text,
        token_count: rendered.token_count.unwrap_or(0),
        file_count: rendered.file_count,
        omitted_files: rendered.omitted_files,
    })
}

pub fn read_file_sync(request: ReadFileRequest) -> Result<ReadFileResponse> {
    read_file_with_context_sync(request, &crate::agent::AgentService::new())
}

/// Reuse one AgentService across reads to retain unchanged source analysis.
pub fn read_file_with_context_sync(
    request: ReadFileRequest,
    service: &crate::agent::AgentService,
) -> Result<ReadFileResponse> {
    let limit = token_limit(request.max_tokens)?;
    if request
        .dependency_tokens
        .is_some_and(|budget| !(256..=65536).contains(&budget))
    {
        bail!("dependency_tokens must be between 256 and 65536");
    }
    let start = request.start_line.unwrap_or(1);
    let lines = request.max_lines.unwrap_or(200);
    if start == 0 || !(1..=1000).contains(&lines) {
        bail!("start_line must be positive and max_lines must be between 1 and 1000");
    }
    let root = request.root.canonicalize()?;
    if !root.is_dir() {
        bail!("root must be a directory");
    }
    if request.path.as_os_str().is_empty()
        || request
            .path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("path must be relative to root without traversal components");
    }
    let mut target = root.clone();
    for component in request.path.components() {
        let name = component.as_os_str().to_string_lossy();
        if name.starts_with('.') {
            bail!("Hidden paths are not available through read_file");
        }
        target.push(component);
        if std::fs::symlink_metadata(&target)?.file_type().is_symlink() {
            bail!("Symlinks are not available through read_file");
        }
    }
    let target = validate_import_path(&root, &target)?;
    if !target.is_file() {
        bail!("read_file requires a regular file");
    }
    if std::fs::metadata(&target)?.len() > MAX_FILE_BYTES {
        bail!("File exceeds the 10 MiB read limit");
    }
    if crate::core::walker::repository_path_is_ignored(&root, &target)? {
        bail!("File is excluded by repository ignore rules");
    }
    if crate::utils::file_ext::is_binary_extension(&target) {
        bail!("Binary files are not available through read_file");
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(&target)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        bail!("File exceeds the 10 MiB read limit");
    }
    let content = String::from_utf8(bytes)?;
    let content = crate::core::secrets::apply_policy(
        &target,
        &content,
        request.secret_scan.unwrap_or(SecretPolicy::Redact),
    )?;
    if crate::utils::file_ext::is_binary_content(content.as_bytes()) {
        bail!("Binary files are not available through read_file");
    }
    let mut candidates = content
        .split_inclusive('\n')
        .skip(start - 1)
        .take(lines + 1);
    let mut selected: Vec<_> = candidates.by_ref().take(lines).collect();
    let more_lines = candidates.next().is_some();
    // Fit complete lines, preserving exact bytes and guaranteeing cursor progress.
    let rendered = fit_rendered(
        &selected,
        Some(limit),
        request.tokenizer.unwrap_or_default(),
        |lines| Ok(lines.concat()),
    )?;
    if !selected.is_empty() && rendered.file_count == 0 {
        bail!("The first requested line exceeds max_tokens; increase the budget");
    }
    let truncated = more_lines || rendered.file_count < selected.len();
    selected.truncate(rendered.file_count);
    let end = if selected.is_empty() {
        None
    } else {
        Some(start.saturating_add(selected.len() - 1))
    };
    let dependency_context = request
        .dependency_tokens
        .map(|budget| {
            let mut context = crate::agent::ContextRequest::new(&root, "");
            context.seeds.push(request.path.clone());
            context.max_tokens = budget;
            context.tokenizer = request.tokenizer.unwrap_or_default();
            context.secret_scan = request.secret_scan.unwrap_or(SecretPolicy::Redact);
            service.build_context(context)
        })
        .transpose()?;
    Ok(ReadFileResponse {
        path: request.path,
        content: rendered.text,
        start_line: start,
        end_line: end,
        next_line: if truncated {
            end.and_then(|end| end.checked_add(1))
        } else {
            None
        },
        truncated,
        token_count: rendered.token_count.unwrap_or(0),
        dependency_context,
    })
}
