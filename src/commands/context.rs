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
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    #[arg(long, default_value = "")]
    pub task: String,
    #[arg(long = "seed")]
    pub seeds: Vec<PathBuf>,
    #[arg(long, default_value = "explain_feature")]
    pub recipe: String,
    #[arg(long)]
    pub recipe_file: Option<PathBuf>,
    #[arg(long, default_value = "8000")]
    pub max_tokens: usize,
    #[arg(long, value_enum, default_value = "o200k-base")]
    pub tokenizer: TokenEncoding,
    #[arg(long, value_enum, default_value = "redact")]
    pub secret_scan: SecretPolicy,
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
        if !(256..=65536).contains(&self.max_tokens) {
            bail!("max_tokens must be between 256 and 65536");
        }
        if self.session_file.is_some() && self.session_id.is_none() {
            bail!("--session-file requires --session-id");
        }
        Ok(())
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

pub fn run_context(args: &ContextArgs) -> Result<()> {
    args.validate()?;
    let book = load_recipe_book(args.recipe_file.as_deref())?;
    let request = ContextRequest {
        root: args.root.clone(),
        task: args.task.clone(),
        seeds: args.seeds.clone(),
        recipe: args.recipe.clone(),
        custom_recipes: book.recipes,
        max_tokens: args.max_tokens,
        tokenizer: args.tokenizer,
        secret_scan: args.secret_scan,
        include_patterns: args.include_patterns.clone(),
        ignore_patterns: args.ignore_patterns.clone(),
        session_id: args.session_id.clone(),
        expected_revision: args.expected_revision.clone(),
        reset_session: args.reset_session,
        base_ref: args.base_ref.clone(),
    };
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
