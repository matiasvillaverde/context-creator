use anyhow::{bail, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
    ReviewChange,
    DebugFailure,
    #[default]
    ExplainFeature,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeDefinition {
    #[serde(default)]
    pub extends: RecipeKind,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    pub include_tests: Option<bool>,
    pub include_config: Option<bool>,
    pub follow_contracts: Option<bool>,
    pub depth: Option<usize>,
    pub max_snippets: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeBook {
    #[serde(default)]
    pub recipes: BTreeMap<String, RecipeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Recipe {
    pub name: String,
    pub kind: RecipeKind,
    pub include_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub include_tests: bool,
    pub include_config: bool,
    pub follow_contracts: bool,
    /// Caller/contract neighborhood steps; zero also disables transitive import traversal.
    pub depth: usize,
    pub max_snippets: usize,
}

pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

pub fn list_recipes(book: &RecipeBook) -> Result<Vec<Recipe>> {
    if book.recipes.len() > 32 {
        bail!("At most 32 custom recipes are supported");
    }
    let mut recipes = Vec::new();
    for (name, kind) in [
        ("review_change", RecipeKind::ReviewChange),
        ("debug_failure", RecipeKind::DebugFailure),
        ("explain_feature", RecipeKind::ExplainFeature),
    ] {
        recipes.push(Recipe {
            name: name.into(),
            kind,
            include_patterns: vec![],
            ignore_patterns: vec![],
            include_tests: true,
            include_config: true,
            follow_contracts: true,
            depth: if kind == RecipeKind::DebugFailure {
                3
            } else {
                2
            },
            max_snippets: 64,
        });
    }
    for (name, definition) in &book.recipes {
        if definition.include_patterns.len() + definition.ignore_patterns.len() > 64 {
            bail!("A recipe supports at most 64 patterns");
        }
        for pattern in definition
            .include_patterns
            .iter()
            .chain(&definition.ignore_patterns)
        {
            crate::core::walker::sanitize_pattern(pattern)?;
        }
        if !valid_name(name) || recipes.iter().any(|recipe| recipe.name == *name) {
            bail!("Invalid or reserved recipe name: {name}");
        }
        if definition.depth.is_some_and(|depth| depth > 5)
            || definition
                .max_snippets
                .is_some_and(|count| !(1..=128).contains(&count))
        {
            bail!("Recipe depth must be at most 5 and max_snippets between 1 and 128");
        }
        let mut recipe = recipes
            .iter()
            .find(|recipe| recipe.kind == definition.extends)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown recipe base"))?;
        recipe.name.clone_from(name);
        recipe
            .include_patterns
            .clone_from(&definition.include_patterns);
        recipe
            .ignore_patterns
            .clone_from(&definition.ignore_patterns);
        recipe.include_tests = definition.include_tests.unwrap_or(recipe.include_tests);
        recipe.include_config = definition.include_config.unwrap_or(recipe.include_config);
        recipe.follow_contracts = definition
            .follow_contracts
            .unwrap_or(recipe.follow_contracts);
        recipe.depth = definition.depth.unwrap_or(recipe.depth);
        recipe.max_snippets = definition.max_snippets.unwrap_or(recipe.max_snippets);
        recipes.push(recipe);
    }
    Ok(recipes)
}
