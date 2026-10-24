//! Reproducible accounting for the generated artifact, separate from stdout.

use super::{budget::RenderedContext, native_tokens::NativeTokenCount, token::TokenCounter};
use crate::cli::Config;
use anyhow::{bail, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct GenerationReport {
    pub schema_version: u32,
    pub version: &'static str,
    pub style: String,
    pub tokenizer: super::token::TokenEncoding,
    pub model: Option<String>,
    pub secret_scan: super::secrets::SecretPolicy,
    pub files_discovered: usize,
    pub files_selected: usize,
    pub files_omitted: usize,
    pub output_bytes: usize,
    pub context_tokens: usize,
    pub local_input_tokens: usize,
    pub output_reserve: usize,
    pub max_tokens: Option<usize>,
    pub native: Option<NativeTokenCount>,
    pub timings_ms: Timings,
}

#[derive(Default, Serialize)]
pub struct Timings {
    pub scan_and_analysis: f64,
    pub selection: f64,
    pub rendering: f64,
    pub native_counting: f64,
    pub accounting: f64,
    /// Through artifact generation; excludes output I/O and LLM execution.
    pub generation_total: f64,
}

impl GenerationReport {
    pub fn new(
        config: &Config,
        rendered: &RenderedContext,
        discovered: usize,
        native: Option<NativeTokenCount>,
        timings_ms: Timings,
    ) -> Result<Self> {
        let counter = TokenCounter::with_encoding(config.get_tokenizer())?;
        let context_tokens = match rendered.token_count {
            Some(count) => count,
            None => counter.count_tokens(&rendered.text)?,
        };
        let local_input_tokens = if let Some(prompt) = config.get_prompt() {
            counter.count_tokens(&format!("{prompt}\n\n{}", rendered.text))?
        } else {
            context_tokens
        };
        Ok(Self {
            schema_version: 1,
            version: env!("CARGO_PKG_VERSION"),
            style: format!("{:?}", config.output_format).to_lowercase(),
            tokenizer: config.get_tokenizer(),
            model: config.model.clone().or_else(|| {
                if config.llm_tool == crate::cli::LlmTool::Ollama {
                    config.ollama_model.clone()
                } else {
                    None
                }
            }),
            secret_scan: config.get_secret_policy(),
            files_discovered: discovered,
            files_selected: rendered.file_count,
            files_omitted: discovered.saturating_sub(rendered.file_count),
            output_bytes: rendered.text.len(),
            context_tokens,
            local_input_tokens,
            output_reserve: if config.get_prompt().is_some() || config.native_token_count {
                config.get_output_reserve()
            } else {
                0
            },
            max_tokens: config.get_effective_max_tokens(),
            native,
            timings_ms,
        })
    }
}

fn absolute_artifact(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid artifact path"))?;
    let name = absolute
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid artifact name"))?;
    Ok(parent.canonicalize()?.join(name))
}

pub fn validate_paths(config: &Config) -> Result<()> {
    if let Some(report) = &config.report {
        let report = absolute_artifact(report)?;
        if config
            .output_file
            .as_ref()
            .is_some_and(|output| absolute_artifact(output).is_ok_and(|output| output == report))
        {
            bail!("--report and --output must use different paths");
        }
        if report.is_dir() {
            bail!("--report must name a file");
        }
    }
    Ok(())
}

/// Avoid ingesting a previous run's context or report into the next run.
pub fn is_artifact(config: &Config, source: &Path) -> bool {
    if config.output_file.is_none() && config.report.is_none() {
        return false;
    }
    let Ok(source) = source.canonicalize() else {
        return false;
    };
    [&config.output_file, &config.report]
        .into_iter()
        .flatten()
        .any(|artifact| absolute_artifact(artifact).is_ok_and(|artifact| artifact == source))
}
