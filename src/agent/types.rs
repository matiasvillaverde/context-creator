use crate::core::{secrets::SecretPolicy, token::TokenEncoding};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

fn default_budget() -> usize {
    8000
}
fn default_recipe() -> String {
    "explain_feature".into()
}
fn redact() -> SecretPolicy {
    SecretPolicy::Redact
}

/// One local, bounded request shared by the library, CLI, and MCP servers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextRequest {
    pub root: PathBuf,
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    /// Files/directories relative to root. Local imports are followed transitively,
    /// with exclusions, unresolved links, and graph/token limits reported in the response.
    pub seeds: Vec<PathBuf>,
    #[serde(default = "default_recipe")]
    pub recipe: String,
    #[serde(default)]
    pub custom_recipes: BTreeMap<String, super::recipes::RecipeDefinition>,
    #[serde(default = "default_budget")]
    pub max_tokens: usize,
    #[serde(default)]
    pub tokenizer: TokenEncoding,
    #[serde(default = "redact")]
    pub secret_scan: SecretPolicy,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    pub session_id: Option<String>,
    /// Last response revision. A mismatch with the session is an error, not an implicit reset.
    pub expected_revision: Option<String>,
    /// Explicitly replace prior session context, including after a lost response.
    #[serde(default)]
    pub reset_session: bool,
    /// Compare this Git commit with the current index and working tree.
    pub base_ref: Option<String>,
}

impl ContextRequest {
    pub fn new(root: impl Into<PathBuf>, task: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            task: task.into(),
            seeds: vec![],
            recipe: default_recipe(),
            custom_recipes: BTreeMap::new(),
            max_tokens: default_budget(),
            tokenizer: TokenEncoding::default(),
            secret_scan: redact(),
            include_patterns: vec![],
            ignore_patterns: vec![],
            session_id: None,
            expected_revision: None,
            reset_session: false,
            base_ref: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Resolved,
    Inferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Location {
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SelectionReason {
    pub kind: String,
    pub detail: String,
    pub confidence: Confidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Location>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    Full,
    Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Snippet {
    pub id: String,
    pub path: String,
    pub symbol: Option<String>,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub representation: Representation,
    pub content_hash: String,
    /// Absent only when this exact content has already been delivered in this session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub reasons: Vec<SelectionReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Omission {
    pub path: String,
    pub symbol: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Gap {
    pub kind: String,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Relation {
    pub from: Location,
    pub to: Location,
    pub kind: String,
    pub confidence: Confidence,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Change {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub ranges: Vec<LineRange>,
    pub removed_symbols: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct IndexStats {
    pub files_scanned: usize,
    pub source_bytes: usize,
    pub declarations_reparsed: usize,
    pub analysis_reused: bool,
}

/// The compact JSON serialization of this value fits max_tokens. MCP envelopes are separate.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextResponse {
    pub schema_version: u32,
    pub recipe: String,
    pub revision: String,
    pub source_revision: String,
    pub previous_revision: Option<String>,
    pub session_id: Option<String>,
    pub stale: bool,
    /// Replace previously delivered context when true; otherwise apply snippets and removed IDs.
    pub reset: bool,
    pub snippets: Vec<Snippet>,
    pub removed: Vec<String>,
    pub omissions: Vec<Omission>,
    pub omitted_snippets: usize,
    pub gaps: Vec<Gap>,
    pub relations: Vec<Relation>,
    pub changes: Vec<Change>,
    pub truncated_metadata: usize,
    pub token_count: usize,
    pub index: IndexStats,
}
