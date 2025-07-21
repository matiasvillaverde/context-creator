#[cfg(test)]
mod test_expansion {
    use context_creator::core::walker::{FileInfo, WalkOptions};
    use context_creator::core::file_expander::expand_file_list;
    use context_creator::core::cache::FileCache;
    use context_creator::cli::Config;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    
    #[test]
    fn test_glob_pattern_with_imports() {
        // Create initial file that matches glob pattern
        let mut files_map = HashMap::new();
        let service_file = FileInfo {
            path: PathBuf::from("/tmp/project/src/services/user_service.py"),
            relative_path: PathBuf::from("src/services/user_service.py"),
            size: 100,
            file_type: context_creator::utils::file_ext::FileType::Python,
            priority: 1.0,
            imports: vec![PathBuf::from("/tmp/project/src/models/user.py")],
            imported_by: vec![],
            function_calls: vec![],
            type_references: vec![],
            exported_functions: vec![],
        };
        files_map.insert(service_file.path.clone(), service_file);
        
        // Create config with trace_imports enabled
        let mut config = Config::default();
        config.trace_imports = true;
        
        // Create walk options with glob pattern
        let walk_options = WalkOptions {
            include_patterns: vec!["src/services/*.py".to_string()],
            ..Default::default()
        };
        
        // Create cache
        let cache = Arc::new(FileCache::new());
        
        // Expand files
        let result = expand_file_list(files_map, &config, &cache, &walk_options).unwrap();
        
        // Check results
        println!("Expanded files:");
        for (path, _) in &result {
            println!("  {}", path.display());
        }
        
        // Should include both the original file and the imported file
        assert_eq!(result.len(), 2, "Should have both service and model files");
        assert!(result.contains_key(&PathBuf::from("/tmp/project/src/services/user_service.py")));
        assert!(result.contains_key(&PathBuf::from("/tmp/project/src/models/user.py")));
    }
}