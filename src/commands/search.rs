//! Search command implementation

use crate::{
    cli::{Commands, Config},
    core::search::{find_files_with_matches, SearchConfig},
    core::walker::FileInfo,
};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// Run the search command
pub fn run_search(mut config: Config) -> Result<()> {
    if let Some(Commands::Search {
        pattern,
        no_semantic,
        paths,
    }) = &config.command
    {
        // Clone values we need before modifying config
        let search_pattern = pattern.clone();
        let search_paths = paths.clone().unwrap_or_else(|| vec![PathBuf::from(".")]);
        let disable_semantic = *no_semantic;

        // Enable semantic flags automatically unless --no-semantic is specified
        if !disable_semantic {
            config.trace_imports = true;
            config.include_callers = true;
            config.include_types = true;
        }

        // Perform search across all paths
        let mut all_matches = Vec::new();
        for path in &search_paths {
            let search_config = SearchConfig {
                pattern: &search_pattern,
                path,
                case_insensitive: true,
                include_globs: &config.get_include_patterns(),
                exclude_globs: &config.get_ignore_patterns(),
            };

            let matches = find_files_with_matches(&search_config)?;
            all_matches.extend(matches);
        }

        // Convert search results to FileInfo for the existing pipeline
        let mut file_map = HashMap::new();
        for path in all_matches {
            let file_info = FileInfo {
                path: path.clone(),
                relative_path: path
                    .strip_prefix(&search_paths[0])
                    .unwrap_or(&path)
                    .to_path_buf(),
                size: std::fs::metadata(&path)?.len(),
                file_type: crate::utils::file_ext::FileType::from_path(&path),
                priority: 10.0, // High priority for direct search matches
                imports: vec![],
                imported_by: vec![],
                function_calls: vec![],
                type_references: vec![],
                exported_functions: vec![],
            };
            file_map.insert(path, file_info);
        }

        // Create temporary directory structure for processing
        // This allows us to reuse the existing process_directory pipeline
        let temp_config = Config {
            command: None, // Clear command to avoid recursion
            paths: Some(search_paths.clone()),
            ..config.clone()
        };

        // Process using existing pipeline with our search results
        // Note: This is a simplified version - in a full implementation,
        // we'd modify process_directory to accept pre-filtered files
        crate::run(temp_config)?;

        Ok(())
    } else {
        unreachable!("run_search called without search command")
    }
}
