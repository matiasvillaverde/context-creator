//! Inspect pinned public source. Never compiles or runs the upstream project.
//! See benchmarks/oss_audit.py for download, sampling, and CLI checks.

use anyhow::{bail, Context, Result};
use context_creator::{
    agent::{AgentService, ContextRequest, ContextResponse, Representation},
    core::{
        secrets::SecretPolicy,
        semantic::{
            get_analyzer_for_file, get_resolver_for_file, symbols::extract_symbols, SemanticContext,
        },
        token::TokenCounter,
    },
    mcp_server::{progressive::read_file_with_context_sync, ReadFileRequest},
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .with_context(|| format!("Missing {key}"))
}

fn inspect_context(
    root: &Path,
    response: &ContextResponse,
    counter: &TokenCounter,
) -> Result<Value> {
    let measured = counter.count_tokens(&serde_json::to_string(response)?)?;
    let mut invalid_source = Vec::new();
    for snippet in &response.snippets {
        let path = root.join(&snippet.path).canonicalize()?;
        if !path.starts_with(root) {
            bail!("Snippet escaped its repository root");
        }
        let source = fs::read_to_string(path)?;
        if let Some(content) = snippet.content.as_deref() {
            if snippet.representation == Representation::Full {
                let range = source
                    .lines()
                    .skip(snippet.start_line.saturating_sub(1))
                    .take(snippet.end_line.saturating_sub(snippet.start_line) + 1)
                    .collect::<Vec<_>>()
                    .join("\n");
                if !source.contains(content)
                    || !range.contains(content.replace("\r\n", "\n").trim())
                {
                    invalid_source.push(snippet.id.clone());
                }
            }
        }
        if snippet.start_line == 0
            || snippet.start_line > snippet.end_line
            || snippet.end_line > source.lines().count().max(1)
        {
            invalid_source.push(snippet.id.clone());
        }
    }
    Ok(json!({
        "exact_token_count": measured == response.token_count,
        "within_budget": measured <= 8192,
        "invalid_source_snippets": invalid_source,
        "response": response,
    }))
}

fn changed_comment(language: &str) -> &'static str {
    match language {
        "Python" | "Ruby" | "Bash" | "Elixir" | "R" | "Julia" | "PowerShell" | "Perl" | "HCL" => {
            "\n# context-creator audit edit\n"
        }
        "Lua" | "Haskell" | "SQL" => "\n-- context-creator audit edit\n",
        "Clojure" => "\n; context-creator audit edit\n",
        "HTML" => "\n<!-- context-creator audit edit -->\n",
        "CSS" => "\n/* context-creator audit edit */\n",
        _ => "\n// context-creator audit edit\n",
    }
}

fn inspect_session(
    root: &Path,
    language: &str,
    relative: &str,
    service: &AgentService,
) -> Result<Value> {
    let path = root.join(relative);
    let source = fs::read_to_string(&path)?;
    let mut request = ContextRequest::new(root, "");
    request.seeds.push(relative.into());
    request.max_tokens = 8192;
    request.secret_scan = SecretPolicy::Off;
    request.session_id = Some("oss-audit".into());
    let initial = service.build_context(request.clone())?;
    request.expected_revision = Some(initial.revision.clone());
    let unchanged = service.build_context(request.clone())?;
    let repeated = unchanged
        .snippets
        .iter()
        .filter(|snippet| {
            snippet.content.is_some()
                && initial.snippets.iter().any(|known| {
                    known.content.is_some()
                        && known.id == snippet.id
                        && known.content_hash == snippet.content_hash
                })
        })
        .count();
    request.expected_revision = Some(unchanged.revision.clone());
    fs::write(&path, format!("{source}{}", changed_comment(language)))?;
    let edited = service.build_context(request.clone());
    fs::write(&path, &source)?;
    let edited = edited?;
    request.expected_revision = Some(initial.revision);
    let old_revision_rejected = service.build_context(request).is_err();
    Ok(json!({
        "previously_delivered_content_omitted": repeated == 0,
        "unchanged_was_reset": unchanged.reset,
        "no_repeat_without_reset": repeated == 0 || unchanged.reset,
        "reset_includes_all_content": !unchanged.reset || unchanged.snippets.iter().all(|snippet| snippet.content.is_some()),
        "reset_removals_empty": !unchanged.reset || unchanged.removed.is_empty(),
        "repeated_content_snippets": repeated,
        "new_content_snippets_on_unchanged_request": unchanged.snippets.iter().filter(|snippet| snippet.content.is_some()).count() - repeated,
        "unchanged_analysis_reused": unchanged.index.analysis_reused,
        "edit_revision_changed": edited.source_revision != unchanged.source_revision,
        "edited_files_reparsed": edited.index.declarations_reparsed,
        "stale_revision_rejected": old_revision_rejected,
    }))
}

fn inspect_sample(
    root: &Path,
    language: &str,
    sample: &Value,
    service: &AgentService,
    counter: &TokenCounter,
) -> Result<Value> {
    let relative = text(sample, "path")?;
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root) {
        bail!("Sample escaped its repository root");
    }
    let source = fs::read_to_string(&path)?;
    let analyzer = get_analyzer_for_file(&path)?.context("No registered analyzer")?;
    let analysis = analyzer.analyze_file(
        &path,
        &source,
        &SemanticContext::new(path.clone(), root.into(), 5),
    )?;
    let resolver = get_resolver_for_file(&path)?;
    let imports = analysis.imports.iter().map(|import| {
        let resolution = resolver.as_ref().map(|resolver| match resolver.resolve_import(&import.module, &path, root) {
            Ok(resolved) => json!({"path": resolved.path, "external": resolved.is_external, "confidence": resolved.confidence}),
            Err(error) => json!({"error": error.to_string()}),
        });
        json!({"module": import.module, "items": import.items, "relative": import.is_relative, "line": import.line, "resolution": resolution})
    }).collect::<Vec<_>>();
    let symbols = extract_symbols(&path, &source)?;
    let mut result = json!({
        "language": language, "path": relative, "analyzer": analyzer.language_name(),
        "source_lines": source.lines().count(), "source_bytes": source.len(),
        "parse_errors": analysis.errors, "symbols": symbols,
        "imports": imports,
        "calls": analysis.function_calls.iter().map(|call| json!({"name":call.name,"module":call.module,"line":call.line})).collect::<Vec<_>>(),
        "types": analysis.type_references.iter().map(|reference| json!({"name":reference.name,"module":reference.module,"line":reference.line,"definition":reference.definition_path,"external":reference.is_external})).collect::<Vec<_>>(),
        "function_definitions": analysis.exported_functions.iter().map(|definition|json!({"name":definition.name,"exported":definition.is_exported,"line":definition.line})).collect::<Vec<_>>(),
    });
    let mut request = ContextRequest::new(root, "");
    request.seeds.push(relative.into());
    request.max_tokens = 8192;
    // Only pinned, public upstream source is loaded by this audit.
    request.secret_scan = SecretPolicy::Off;
    let context_started = Instant::now();
    match service.build_context(request.clone()) {
        Ok(response) => result["context"] = inspect_context(root, &response, counter)?,
        Err(error) => result["context"] = json!({"error": error.to_string()}),
    }
    result["context_seconds"] = json!(context_started.elapsed().as_secs_f64());
    let read = ReadFileRequest {
        root: root.into(),
        path: relative.into(),
        start_line: Some(1),
        max_lines: Some(40),
        max_tokens: Some(2048),
        dependency_tokens: Some(8192),
        tokenizer: None,
        secret_scan: Some(SecretPolicy::Off),
    };
    match read_file_with_context_sync(read, service) {
        Ok(response) => {
            let dependency = response
                .dependency_context
                .as_ref()
                .context("Dependency context absent")?;
            result["read_file"] = json!({"page_matches_source": source.contains(&response.content),
                "page_within_budget": response.token_count <= 2048,
                "context": inspect_context(root, dependency, counter)?});
        }
        Err(error) => result["read_file"] = json!({"error": error.to_string()}),
    }
    Ok(result)
}

fn main() -> Result<()> {
    let arguments = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if !(3..=5).contains(&arguments.len()) {
        bail!("Usage: audit_language_support MANIFEST CACHE OUTPUT_DIRECTORY [LANGUAGE] [full|sessions|symbols]");
    }
    let mode = arguments
        .get(4)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "full".into());
    if !["full", "sessions", "symbols"].contains(&mode.as_str()) {
        bail!("Unknown audit mode");
    }
    let manifest: Value = serde_json::from_str(&fs::read_to_string(&arguments[0])?)?;
    let projects = manifest["projects"]
        .as_array()
        .context("Missing projects")?;
    let cache = arguments[1].canonicalize()?;
    fs::create_dir_all(&arguments[2])?;
    let counter = TokenCounter::new()?;
    for project in projects {
        let language = text(project, "language")?;
        if arguments
            .get(3)
            .is_some_and(|filter| filter.as_os_str() != language)
        {
            continue;
        }
        let root = cache.join(text(project, "cache_key")?).canonicalize()?;
        if !root.starts_with(&cache) {
            bail!("Project escaped its source cache");
        }
        let service = AgentService::new();
        let samples = project["samples"].as_array().context("Missing samples")?;
        for (index, sample) in samples.iter().enumerate() {
            if mode == "sessions" && index > 0 {
                break;
            }
            let path = root.join(text(sample, "path")?).canonicalize()?;
            if !path.starts_with(&root) {
                bail!("Sample escaped its repository root");
            }
            let started = Instant::now();
            let inspected = match mode.as_str() {
                "sessions" => inspect_session(&root, language, text(sample, "path")?, &service),
                "symbols" => {
                    let path = root.join(text(sample, "path")?);
                    extract_symbols(&path, &fs::read_to_string(&path)?)
                        .map(|symbols| json!({"symbols":symbols}))
                        .map_err(Into::into)
                }
                _ => inspect_sample(&root, language, sample, &service, &counter),
            };
            let mut result = match inspected {
                Ok(result) => result,
                Err(error) => {
                    json!({"language":language,"path":sample["path"],"error":format!("{error:#}")})
                }
            };
            result["seconds"] = json!(started.elapsed().as_secs_f64());
            if mode == "full" && index == 0 {
                result["session"] =
                    match inspect_session(&root, language, text(sample, "path")?, &service) {
                        Ok(session) => session,
                        Err(error) => json!({"error": error.to_string()}),
                    };
            }
            result["repository"] = project["repository"].clone();
            result["commit"] = project["commit"].clone();
            result["path"] = sample["path"].clone();
            result["mode"] = json!(mode);
            let key = language
                .replace('+', "p")
                .replace('#', "sharp")
                .to_lowercase();
            fs::write(
                arguments[2].join(format!(
                    "{key}-{index}.{}.json",
                    if mode == "full" { "semantic" } else { &mode }
                )),
                serde_json::to_string_pretty(&result)?,
            )?;
            eprintln!(
                "{language} {}: {:.2}s",
                text(sample, "path")?,
                started.elapsed().as_secs_f64()
            );
        }
    }
    Ok(())
}
