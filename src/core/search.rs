//! Core search functionality using ripgrep

use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Configuration for search operations
pub struct SearchConfig<'a> {
    pub pattern: &'a str,
    pub path: &'a Path,
    pub case_insensitive: bool,
    pub include_globs: &'a [String],
    pub exclude_globs: &'a [String],
}

/// Find files containing matches for the given pattern
pub fn find_files_with_matches(config: &SearchConfig) -> Result<Vec<PathBuf>> {
    let mut matches = Vec::new();

    // Build the walker with include/exclude patterns
    let mut builder = WalkBuilder::new(config.path);
    builder.hidden(false);

    // Handle both include and exclude patterns using OverrideBuilder
    if !config.include_globs.is_empty() || !config.exclude_globs.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(config.path);

        // If we have include patterns, add them
        if !config.include_globs.is_empty() {
            for pattern in config.include_globs {
                overrides.add(pattern)?;
            }
        } else if !config.exclude_globs.is_empty() {
            // If we only have exclude patterns, include everything first
            overrides.add("**/*")?;
        }

        // Add exclude patterns with ! prefix
        for pattern in config.exclude_globs {
            let exclude_pattern = format!("!{pattern}");
            overrides.add(&exclude_pattern)?;
        }

        builder.overrides(overrides.build()?);
    }

    // Walk files and search
    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Read file content
        if let Ok(content) = std::fs::read_to_string(path) {
            // Perform search
            let found = if config.case_insensitive {
                content
                    .to_lowercase()
                    .contains(&config.pattern.to_lowercase())
            } else {
                content.contains(config.pattern)
            };

            if found {
                matches.push(path.to_path_buf());
            }
        }
    }

    Ok(matches)
}
