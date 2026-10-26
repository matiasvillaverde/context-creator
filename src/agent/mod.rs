//! Local context tools shared by Rust consumers, CLI processes, and MCP clients.
//! Keep an AgentService alive to reuse analysis and revision-checked session state.
mod contracts;
mod impact;
mod index;
pub mod recipes;
mod select;
mod sessions;
mod types;

pub use recipes::{list_recipes, Recipe, RecipeBook, RecipeDefinition, RecipeKind};
pub use sessions::{load_checkpoint, SessionState};
pub use types::*;

use crate::{cli::Config, core::token::TokenCounter};
use anyhow::{bail, Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Mutex},
};

pub(crate) fn digest(bytes: &[u8]) -> String {
    // Git's blob hash is deterministic across processes and platforms.
    git2::Oid::hash_object(git2::ObjectType::Blob, bytes)
        .map(|oid| oid.to_string())
        .unwrap_or_else(|_| "hash-unavailable".into())
}

#[derive(Default)]
struct State {
    snapshots: BTreeMap<String, Arc<index::Snapshot>>,
    sessions: BTreeMap<String, SessionState>,
}

/// Blocking service. Async integrations must call it through a blocking worker.
#[derive(Default)]
pub struct AgentService {
    state: Mutex<State>,
}

impl std::fmt::Debug for AgentService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentService")
            .finish_non_exhaustive()
    }
}

impl AgentService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_context(&self, request: ContextRequest) -> Result<ContextResponse> {
        self.build(request, None)
    }

    /// Trusted local checkpoint path; intentionally absent from the MCP schema.
    pub fn build_context_with_checkpoint(
        &self,
        request: ContextRequest,
        checkpoint: &Path,
    ) -> Result<ContextResponse> {
        self.build(request, Some(checkpoint))
    }

    fn build(
        &self,
        mut request: ContextRequest,
        checkpoint: Option<&Path>,
    ) -> Result<ContextResponse> {
        sessions::validate_request_session(&request)?;
        if request.include_patterns.len() + request.ignore_patterns.len() > 64 {
            bail!("A context request supports at most 64 patterns");
        }
        if request.task.len() > 16384
            || request.seeds.len() > 64
            || !(256..=65536).contains(&request.max_tokens)
        {
            bail!("Task is limited to 16 KiB, seeds to 64, and max_tokens to 256..=65536");
        }
        let recipe = list_recipes(&RecipeBook {
            recipes: request.custom_recipes.clone(),
        })?
        .into_iter()
        .find(|recipe| recipe.name == request.recipe)
        .context("Unknown context recipe")?;
        if recipe.kind == RecipeKind::ReviewChange && request.base_ref.is_none() {
            bail!("review_change requires base_ref");
        }
        if request.task.trim().is_empty() && request.seeds.is_empty() && request.base_ref.is_none()
        {
            bail!("Provide a task, seeds, or base_ref");
        }
        let root = request.root.canonicalize()?;
        if !root.is_dir() {
            bail!("Agent root must be a directory");
        }
        request.root = root.clone();
        let mut config = Config {
            paths: Some(vec![root.clone()]),
            trace_imports: true,
            include_callers: true,
            include_types: true,
            secret_scan: Some(request.secret_scan),
            ignore: Some(
                request
                    .ignore_patterns
                    .iter()
                    .chain(recipe.ignore_patterns.iter())
                    .cloned()
                    .collect(),
            ),
            quiet: true,
            ..Config::default()
        };
        config.semantic_depth = 5;
        config.validate()?;
        for pattern in request
            .include_patterns
            .iter()
            .chain(recipe.include_patterns.iter())
        {
            crate::core::walker::sanitize_pattern(pattern)?;
        }
        let scope = digest(
            serde_json::to_string(&(
                root.to_string_lossy(),
                request.secret_scan,
                &config.ignore,
                request.tokenizer,
            ))?
            .as_bytes(),
        );
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Agent service lock is unavailable"))?;
        let session_key = request
            .session_id
            .as_ref()
            .map(|name| format!("{scope}:{name}"));
        if session_key
            .as_ref()
            .is_some_and(|key| state.sessions.len() >= 128 && !state.sessions.contains_key(key))
        {
            bail!("Session limit reached; create a new AgentService");
        }
        let mut checkpoint_lock = None;
        if let Some(checkpoint) = checkpoint {
            if session_key.is_none() {
                bail!("A checkpoint requires session_id");
            }
            let parent = checkpoint
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .canonicalize()?;
            if parent.starts_with(&root) {
                bail!("Store session checkpoints outside the source repository");
            }
            checkpoint_lock = Some(sessions::CheckpointLock::acquire(checkpoint)?);
            if let Some(stored) = load_checkpoint(checkpoint)? {
                if stored.scope != scope || request.session_id.as_ref() != Some(&stored.session_id)
                {
                    bail!("Checkpoint repository or content policy does not match");
                }
                if let Some(key) = &session_key {
                    state.sessions.insert(key.clone(), stored);
                }
            }
        }
        let previous = session_key
            .as_ref()
            .and_then(|key| state.sessions.get(key))
            .filter(|_| !request.reset_session)
            .cloned();
        match (&previous, &request.expected_revision) {
            (Some(previous), Some(expected)) if previous.revision != *expected => {
                bail!("Session revision conflict; use the last delivered revision")
            }
            (Some(_), None) => bail!("Existing session requires expected_revision"),
            (None, Some(_)) => {
                bail!("Session is missing or expired; start without expected_revision")
            }
            _ => {}
        }
        let (snapshot, stats) =
            index::snapshot(&root, &config, state.snapshots.get(&scope).map(Arc::as_ref))?;
        let changes = request
            .base_ref
            .as_ref()
            .map(|baseline| impact::changes(&root, baseline, &snapshot))
            .transpose()?
            .unwrap_or_default();
        let selection = select::select(&snapshot, &request, &recipe, &changes)?;
        let counter = TokenCounter::with_encoding(request.tokenizer)?;
        let mut response = ContextResponse {
            schema_version: 1,
            recipe: recipe.name,
            revision: digest(&serde_json::to_vec(&(
                &scope,
                &snapshot.revision,
                &request,
                previous.as_ref().map(|session| &session.revision),
            ))?),
            source_revision: snapshot.revision.clone(),
            previous_revision: previous.as_ref().map(|session| session.revision.clone()),
            session_id: request.session_id.clone(),
            stale: previous
                .as_ref()
                .is_some_and(|previous| previous.source_revision != snapshot.revision),
            reset: previous.is_none(),
            snippets: vec![],
            removed: vec![],
            omissions: vec![],
            omitted_snippets: selection.candidates.len(),
            gaps: vec![],
            relations: vec![],
            changes: vec![],
            truncated_metadata: 0,
            token_count: 0,
            index: stats,
        };
        if !fits(&mut response, &counter, request.max_tokens)? {
            bail!("Budget too small for context metadata");
        }
        let mut ranges = Vec::<(String, (usize, usize))>::new();
        let bounds = selection
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.snippet.id.as_str(),
                    (candidate.snippet.path.as_str(), candidate.bytes),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for candidate in &selection.candidates {
            if response.snippets.len() >= recipe.max_snippets {
                break;
            }
            if ranges.iter().any(|(path, (start, end))| {
                *path == candidate.snippet.path
                    && *start <= candidate.bytes.0
                    && *end >= candidate.bytes.1
            }) {
                continue;
            }
            // A complete enclosing declaration also supplies its previously selected methods.
            // Temporarily remove those methods so duplicated source does not consume the budget.
            let mut covered = Vec::new();
            let existing = std::mem::take(&mut response.snippets);
            for (index, snippet) in existing.into_iter().enumerate() {
                if bounds
                    .get(snippet.id.as_str())
                    .is_some_and(|(path, (start, end))| {
                        *path == candidate.snippet.path
                            && candidate.bytes.0 <= *start
                            && candidate.bytes.1 >= *end
                    })
                {
                    covered.push((index, snippet));
                } else {
                    response.snippets.push(snippet);
                }
            }
            let mut item = candidate.snippet.clone();
            let full_hash = item.content_hash.clone();
            if previous
                .as_ref()
                .and_then(|session| session.delivered.get(&item.id))
                == Some(&item.content_hash)
            {
                item.content = None;
            }
            response.snippets.push(item);
            let mut accepted = fits(&mut response, &counter, request.max_tokens)?;
            if !accepted {
                response.snippets.pop();
                for (index, snippet) in covered {
                    response.snippets.insert(index, snippet);
                }
                if let Some(signature) = &candidate.signature {
                    let mut item = candidate.snippet.clone();
                    item.representation = Representation::Signature;
                    item.content_hash = digest(signature.as_bytes());
                    item.content = Some(signature.clone());
                    if previous
                        .as_ref()
                        .and_then(|session| session.delivered.get(&item.id))
                        == Some(&item.content_hash)
                    {
                        item.content = None;
                    }
                    response.snippets.push(item);
                    accepted = fits(&mut response, &counter, request.max_tokens)?;
                    if !accepted {
                        response.snippets.pop();
                    }
                }
            }
            if accepted
                && candidate.snippet.representation == Representation::Full
                && response
                    .snippets
                    .last()
                    .is_some_and(|snippet| snippet.content_hash == full_hash)
            {
                ranges.push((candidate.snippet.path.clone(), candidate.bytes));
            }
        }
        let selected = response
            .snippets
            .iter()
            .map(|snippet| snippet.id.clone())
            .collect::<BTreeSet<_>>();
        response.omitted_snippets = selection.candidates.len().saturating_sub(selected.len());
        if let Some(previous) = &previous {
            response.removed = previous
                .delivered
                .keys()
                .filter(|id| !selected.contains(*id))
                .cloned()
                .collect();
            if !fits(&mut response, &counter, request.max_tokens)? {
                // A bounded response may not fit every removal. Explicitly replace context instead.
                response.reset = true;
                response.removed.clear();
                for snippet in &mut response.snippets {
                    if snippet.content.is_none() {
                        if let Some(candidate) = selection
                            .candidates
                            .iter()
                            .find(|candidate| candidate.snippet.id == snippet.id)
                        {
                            snippet.content = match snippet.representation {
                                Representation::Full => candidate.snippet.content.clone(),
                                Representation::Signature => candidate.signature.clone(),
                            };
                        }
                    }
                }
                while !fits(&mut response, &counter, request.max_tokens)?
                    && !response.snippets.is_empty()
                {
                    response.snippets.pop();
                }
                response.omitted_snippets = selection
                    .candidates
                    .len()
                    .saturating_sub(response.snippets.len());
            }
        }
        let omitted = selection
            .candidates
            .iter()
            .filter(|candidate| {
                !response
                    .snippets
                    .iter()
                    .any(|snippet| snippet.id == candidate.snippet.id)
            })
            .map(|candidate| Omission {
                path: candidate.snippet.path.clone(),
                symbol: candidate.snippet.symbol.clone(),
                reason:
                    "Budget, recipe snippet limit, or already covered by an enclosing declaration"
                        .into(),
            })
            .collect::<Vec<_>>();
        let signatures = response
            .snippets
            .iter()
            .filter(|snippet| {
                snippet.kind == "type" && snippet.representation == Representation::Signature
            })
            .map(|snippet| Gap {
                kind: "type_body_omitted".into(),
                path: Some(snippet.path.clone()),
                line: Some(snippet.start_line),
                detail: "Type rendered as a signature to fit the budget; read the source for omitted fields and members".into(),
            })
            .collect::<Vec<_>>();
        fit_metadata(
            &mut response,
            signatures.into_iter().chain(selection.gaps).collect(),
            |response| &mut response.gaps,
            &counter,
            request.max_tokens,
        )?;
        fit_metadata(
            &mut response,
            changes,
            |response| &mut response.changes,
            &counter,
            request.max_tokens,
        )?;
        fit_metadata(
            &mut response,
            selection.relations,
            |response| &mut response.relations,
            &counter,
            request.max_tokens,
        )?;
        fit_metadata(
            &mut response,
            omitted,
            |response| &mut response.omissions,
            &counter,
            request.max_tokens,
        )?;
        while !fits(&mut response, &counter, request.max_tokens)? {
            if response.omissions.pop().is_some()
                || response.relations.pop().is_some()
                || response.changes.pop().is_some()
                || response.gaps.pop().is_some()
            {
                response.truncated_metadata += 1;
            } else if response.snippets.pop().is_some() {
                response.reset = true;
                response.removed.clear();
                response.omitted_snippets += 1;
                for snippet in &mut response.snippets {
                    if snippet.content.is_none() {
                        if let Some(candidate) = selection
                            .candidates
                            .iter()
                            .find(|candidate| candidate.snippet.id == snippet.id)
                        {
                            snippet.content = match snippet.representation {
                                Representation::Full => candidate.snippet.content.clone(),
                                Representation::Signature => candidate.signature.clone(),
                            };
                        }
                    }
                }
            } else {
                bail!("Budget too small for context response");
            }
        }
        if let Some(key) = session_key {
            let session = SessionState {
                schema_version: 1,
                scope: scope.clone(),
                session_id: request.session_id.clone().context("Missing session ID")?,
                revision: response.revision.clone(),
                source_revision: snapshot.revision.clone(),
                delivered: response
                    .snippets
                    .iter()
                    .map(|snippet| (snippet.id.clone(), snippet.content_hash.clone()))
                    .collect(),
            };
            if let Some(checkpoint) = checkpoint {
                sessions::save_checkpoint(checkpoint, &session)?;
            }
            state.sessions.insert(key, session);
        }
        if state.snapshots.len() >= 2 && !state.snapshots.contains_key(&scope) {
            if let Some(key) = state.snapshots.keys().next().cloned() {
                state.snapshots.remove(&key);
            }
        }
        state.snapshots.insert(scope, snapshot);
        drop(checkpoint_lock);
        Ok(response)
    }
}

fn fit_metadata<T: Clone>(
    response: &mut ContextResponse,
    values: Vec<T>,
    field: fn(&mut ContextResponse) -> &mut Vec<T>,
    counter: &TokenCounter,
    budget: usize,
) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    let original = field(response).len();
    let truncated = response.truncated_metadata;
    field(response).extend(values.iter().cloned());
    if fits(response, counter, budget)? {
        return Ok(());
    }
    field(response).truncate(original);
    // Fit an ordered prefix in logarithmic recounts. Deep graphs must not
    // repeatedly tokenize an almost-full response once for every omitted edge.
    let mut low = 0;
    let mut high = values.len();
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        response.truncated_metadata = truncated + values.len() - middle;
        field(response).extend(values[..middle].iter().cloned());
        if fits(response, counter, budget)? {
            low = middle;
        } else {
            high = middle - 1;
        }
        field(response).truncate(original);
    }
    response.truncated_metadata = truncated + values.len() - low;
    field(response).extend(values.into_iter().take(low));
    Ok(())
}

fn fits(response: &mut ContextResponse, counter: &TokenCounter, budget: usize) -> Result<bool> {
    for _ in 0..12 {
        let count = counter.count_tokens(&serde_json::to_string(response)?)?;
        if count == response.token_count {
            return Ok(count <= budget);
        }
        response.token_count = count;
    }
    bail!("Token accounting did not converge")
}
