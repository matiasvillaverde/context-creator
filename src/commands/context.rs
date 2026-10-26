use crate::agent::{AgentService, ContextRequest, RecipeBook};
use crate::core::{secrets::SecretPolicy, token::TokenEncoding};
use anyhow::{bail, Result};
use clap::Args;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Args, Debug, Clone)]
pub struct ContextArgs {
    /// Repository root (inherits the configured directory, otherwise current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long, default_value = "")]
    pub task: String,
    #[arg(long = "seed")]
    pub seeds: Vec<PathBuf>,
    #[arg(long, default_value = "explain_feature")]
    pub recipe: String,
    #[arg(long)]
    pub recipe_file: Option<PathBuf>,
    /// Maximum compact JSON tokens (inherits CLI/TOML; default: 8000, range: 256..=65536)
    #[arg(long)]
    pub max_tokens: Option<usize>,
    /// Token encoding (inherits CLI/TOML; default: o200k-base)
    #[arg(long, value_enum)]
    pub tokenizer: Option<TokenEncoding>,
    /// Secret policy (inherits CLI/TOML; default: redact)
    #[arg(long, value_enum)]
    pub secret_scan: Option<SecretPolicy>,
    #[arg(long = "include")]
    pub include_patterns: Vec<String>,
    #[arg(long = "ignore")]
    pub ignore_patterns: Vec<String>,
    #[arg(long)]
    pub session_id: Option<String>,
    #[arg(long)]
    pub expected_revision: Option<String>,
    #[arg(long)]
    pub reset_session: bool,
    /// Explicit checkpoint outside the source root; persists hashes, never source contents.
    #[arg(long)]
    pub session_file: Option<PathBuf>,
    #[arg(long)]
    pub base_ref: Option<String>,
}

impl ContextArgs {
    pub fn validate(&self) -> Result<()> {
        if self
            .max_tokens
            .is_some_and(|budget| !(256..=65536).contains(&budget))
        {
            bail!("max_tokens must be between 256 and 65536");
        }
        if self.session_file.is_some() && self.session_id.is_none() {
            bail!("--session-file requires --session-id");
        }
        Ok(())
    }

    /// Resolve command flags over the already merged CLI/TOML configuration.
    pub fn to_request(&self, config: &crate::Config) -> Result<ContextRequest> {
        self.validate()?;
        let root = if let Some(root) = &self.root {
            root.clone()
        } else {
            let roots = config.get_directories();
            if roots.len() != 1 {
                bail!("context requires one repository root; use --root");
            }
            roots
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing context root"))?
        };
        let mut request = ContextRequest::new(root, self.task.clone());
        request.seeds.clone_from(&self.seeds);
        request.recipe.clone_from(&self.recipe);
        request.max_tokens = self
            .max_tokens
            .or_else(|| config.get_effective_max_tokens())
            .unwrap_or(request.max_tokens);
        if !(256..=65536).contains(&request.max_tokens) {
            bail!("context max_tokens must be between 256 and 65536; override the configured budget with context --max-tokens");
        }
        request.tokenizer = self.tokenizer.unwrap_or_else(|| config.get_tokenizer());
        request.secret_scan = self
            .secret_scan
            .or(config.secret_scan)
            .unwrap_or(SecretPolicy::Redact);
        request.include_patterns = if self.include_patterns.is_empty() {
            config.get_include_patterns()
        } else {
            self.include_patterns.clone()
        };
        request.ignore_patterns = if self.ignore_patterns.is_empty() {
            config.get_ignore_patterns()
        } else {
            self.ignore_patterns.clone()
        };
        request.session_id.clone_from(&self.session_id);
        request
            .expected_revision
            .clone_from(&self.expected_revision);
        request.reset_session = self.reset_session;
        request.base_ref.clone_from(&self.base_ref);
        Ok(request)
    }
}

pub fn load_recipe_book(path: Option<&Path>) -> Result<RecipeBook> {
    let Some(path) = path else {
        return Ok(RecipeBook::default());
    };
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        bail!("Recipe file must be a regular file");
    }
    let mut text = String::new();
    file.take(65537).read_to_string(&mut text)?;
    if text.len() > 65536 {
        bail!("Recipe file exceeds 64 KiB");
    }
    Ok(toml::from_str(&text)?)
}

pub fn run_context(args: &ContextArgs, config: &crate::Config) -> Result<()> {
    let mut request = args.to_request(config)?;
    let book = load_recipe_book(args.recipe_file.as_deref())?;
    request.custom_recipes = book.recipes;
    let service = AgentService::new();
    let response = if let Some(checkpoint) = &args.session_file {
        service.build_context_with_checkpoint(request, checkpoint)?
    } else {
        service.build_context(request)?
    };
    // Compact JSON is the budgeted wire representation; no uncounted trailing newline.
    print!("{}", serde_json::to_string(&response)?);
    Ok(())
}
