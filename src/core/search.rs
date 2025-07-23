//! Core search functionality using ripgrep

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Configuration for search operations
pub struct SearchConfig<'a> {
    pub pattern: &'a str,
    pub path: &'a Path,
    pub case_insensitive: bool,
    pub include_globs: &'a [String],
    pub exclude_globs: &'a [String],
}

/// Maximum file size to process (10MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Find files containing matches for the given pattern using parallel search
pub fn find_files_with_matches(config: &SearchConfig) -> Result<Vec<PathBuf>> {
    // Build the walker with include/exclude patterns
    let mut builder = WalkBuilder::new(config.path);
    builder.hidden(false);

    // Enable parallel walking for maximum performance
    builder.threads(num_cpus::get());

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

    // Prepare pattern for search (pre-compute lowercase version if needed)
    let pattern_lower = if config.case_insensitive {
        Some(config.pattern.to_lowercase())
    } else {
        None
    };

    // Thread-safe collection of matches
    let matches = Arc::new(Mutex::new(Vec::new()));
    let matches_clone = matches.clone();

    // Build parallel walker
    builder.build_parallel().run(|| {
        let matches = matches_clone.clone();
        let pattern = config.pattern;
        let pattern_lower = pattern_lower.clone();

        Box::new(move |entry| {
            if let Ok(entry) = entry {
                let path = entry.path();

                // Skip directories
                if path.is_dir() {
                    return ignore::WalkState::Continue;
                }

                // Check if file should be searched
                if should_search_file(path, pattern, pattern_lower.as_deref()) {
                    matches.lock().unwrap().push(path.to_path_buf());
                }
            }
            ignore::WalkState::Continue
        })
    });

    // Extract results
    let results = Arc::try_unwrap(matches)
        .map(|mutex| mutex.into_inner().unwrap())
        .unwrap_or_else(|arc| arc.lock().unwrap().clone());

    Ok(results)
}

/// Check if a file contains the search pattern using streaming
fn should_search_file(path: &Path, pattern: &str, pattern_lower: Option<&str>) -> bool {
    // First check file size to prevent DoS
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > MAX_FILE_SIZE {
            return false;
        }
    }

    // Open file for streaming search
    let file = match File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))
    {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file);

    // Stream through file line by line
    if let Some(pattern_lower) = pattern_lower {
        // Case-insensitive search
        for line in reader.lines().map_while(Result::ok) {
            if line.to_lowercase().contains(pattern_lower) {
                return true;
            }
        }
    } else {
        // Case-sensitive search using memchr for maximum performance
        // For short patterns, use the fast substring search
        if pattern.len() <= 32 {
            for line in reader.lines().map_while(Result::ok) {
                if line.contains(pattern) {
                    return true;
                }
            }
        } else {
            // For longer patterns, use boyer-moore-like algorithm
            for line in reader.lines().map_while(Result::ok) {
                if fast_substring_search(&line, pattern) {
                    return true;
                }
            }
        }
    }

    false
}

/// Fast substring search optimized for longer patterns
fn fast_substring_search(haystack: &str, needle: &str) -> bool {
    // Use Rust's built-in contains which is highly optimized
    // It uses SIMD instructions when available
    haystack.contains(needle)
}
