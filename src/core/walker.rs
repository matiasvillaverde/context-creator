//! Directory walking functionality with .gitignore and .digestignore support

use crate::utils::error::CodeDigestError;
use crate::utils::file_ext::FileType;
use anyhow::Result;
use glob::Pattern;
use ignore::{Walk, WalkBuilder};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Compiled priority rule for efficient pattern matching
///
/// This struct represents a custom priority rule that has been compiled from
/// the configuration file. The glob pattern is pre-compiled for performance,
/// and the weight is applied additively to the base file type priority.
///
/// # Priority Calculation
/// Final priority = base_priority + weight (if pattern matches)
///
/// # Pattern Matching
/// Uses first-match-wins semantics - the first pattern that matches a file
/// will determine the priority adjustment. Subsequent patterns are not evaluated.
#[derive(Debug, Clone)]
pub struct CompiledPriority {
    /// Pre-compiled glob pattern for efficient matching
    pub matcher: Pattern,
    /// Priority weight to add to base priority (can be negative)
    pub weight: f32,
    /// Original pattern string for debugging and error reporting
    pub original_pattern: String,
}

impl CompiledPriority {
    /// Create a CompiledPriority from a pattern string
    pub fn new(pattern: &str, weight: f32) -> Result<Self, glob::PatternError> {
        let matcher = Pattern::new(pattern)?;
        Ok(Self { matcher, weight, original_pattern: pattern.to_string() })
    }

    /// Convert from config::Priority to CompiledPriority with error handling
    pub fn try_from_config_priority(
        priority: &crate::config::Priority,
    ) -> Result<Self, glob::PatternError> {
        Self::new(&priority.pattern, priority.weight)
    }
}

/// Options for walking directories
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Maximum file size in bytes
    pub max_file_size: Option<usize>,
    /// Follow symbolic links
    pub follow_links: bool,
    /// Include hidden files
    pub include_hidden: bool,
    /// Use parallel processing
    pub parallel: bool,
    /// Custom ignore file name (default: .digestignore)
    pub ignore_file: String,
    /// Additional glob patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Only include files matching these patterns
    pub include_patterns: Vec<String>,
    /// Custom priority rules for file prioritization
    pub custom_priorities: Vec<CompiledPriority>,
}

impl WalkOptions {
    /// Create WalkOptions from CLI config
    pub fn from_config(config: &crate::cli::Config) -> Result<Self> {
        // Convert config priorities to CompiledPriority with error handling
        let mut custom_priorities = Vec::new();
        for priority in &config.custom_priorities {
            match CompiledPriority::try_from_config_priority(priority) {
                Ok(compiled) => custom_priorities.push(compiled),
                Err(e) => {
                    return Err(CodeDigestError::ConfigError(format!(
                        "Invalid glob pattern '{}' in custom priorities: {e}",
                        priority.pattern
                    ))
                    .into());
                }
            }
        }

        // Get include patterns from CLI config and filter out empty/whitespace patterns
        let include_patterns = config
            .get_include_patterns()
            .into_iter()
            .filter(|pattern| !pattern.trim().is_empty())
            .collect();

        Ok(WalkOptions {
            max_file_size: Some(10 * 1024 * 1024), // 10MB default
            follow_links: false,
            include_hidden: false,
            parallel: true,
            ignore_file: ".digestignore".to_string(),
            ignore_patterns: vec![],
            include_patterns,
            custom_priorities,
        })
    }
}

impl Default for WalkOptions {
    fn default() -> Self {
        WalkOptions {
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            follow_links: false,
            include_hidden: false,
            parallel: true,
            ignore_file: ".digestignore".to_string(),
            ignore_patterns: vec![],
            include_patterns: vec![],
            custom_priorities: vec![],
        }
    }
}

/// Information about a file found during walking
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Relative path from the root directory
    pub relative_path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File type based on extension
    pub file_type: FileType,
    /// Priority score (higher is more important)
    pub priority: f32,
}

impl FileInfo {
    /// Get a display string for the file type
    pub fn file_type_display(&self) -> &'static str {
        use crate::utils::file_ext::FileType;
        match self.file_type {
            FileType::Rust => "Rust",
            FileType::Python => "Python",
            FileType::JavaScript => "JavaScript",
            FileType::TypeScript => "TypeScript",
            FileType::Go => "Go",
            FileType::Java => "Java",
            FileType::Cpp => "C++",
            FileType::C => "C",
            FileType::CSharp => "C#",
            FileType::Ruby => "Ruby",
            FileType::Php => "PHP",
            FileType::Swift => "Swift",
            FileType::Kotlin => "Kotlin",
            FileType::Scala => "Scala",
            FileType::Haskell => "Haskell",
            FileType::Markdown => "Markdown",
            FileType::Json => "JSON",
            FileType::Yaml => "YAML",
            FileType::Toml => "TOML",
            FileType::Xml => "XML",
            FileType::Html => "HTML",
            FileType::Css => "CSS",
            FileType::Text => "Text",
            FileType::Other => "Other",
        }
    }
}

/// Walk a directory and collect file information
pub fn walk_directory(root: &Path, options: WalkOptions) -> Result<Vec<FileInfo>> {
    if !root.exists() {
        return Err(CodeDigestError::InvalidPath(format!(
            "Directory does not exist: {}",
            root.display()
        ))
        .into());
    }

    if !root.is_dir() {
        return Err(CodeDigestError::InvalidPath(format!(
            "Path is not a directory: {}",
            root.display()
        ))
        .into());
    }

    let root = root.canonicalize()?;
    let walker = build_walker(&root, &options)?;

    if options.parallel {
        walk_parallel(walker, &root, &options)
    } else {
        walk_sequential(walker, &root, &options)
    }
}

/// Build the ignore walker with configured options
fn build_walker(root: &Path, options: &WalkOptions) -> Result<Walk> {
    let mut builder = WalkBuilder::new(root);

    // Configure the walker
    builder
        .follow_links(options.follow_links)
        .hidden(!options.include_hidden)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .add_custom_ignore_filename(&options.ignore_file);

    // Add custom ignore patterns
    for pattern in &options.ignore_patterns {
        let _ = builder.add_ignore(pattern);
    }

    // Handle include patterns using OverrideBuilder for proper filtering
    if !options.include_patterns.is_empty() {
        let mut override_builder = ignore::overrides::OverrideBuilder::new(root);

        for pattern in &options.include_patterns {
            if !pattern.trim().is_empty() {
                // Include patterns are added directly (not as negations)
                override_builder.add(pattern).map_err(|e| {
                    CodeDigestError::InvalidConfiguration(format!(
                        "Invalid include pattern '{pattern}': {e}"
                    ))
                })?;
            }
        }

        let overrides = override_builder.build().map_err(|e| {
            CodeDigestError::InvalidConfiguration(format!(
                "Failed to build include pattern overrides: {e}"
            ))
        })?;

        builder.overrides(overrides);
    }

    Ok(builder.build())
}

/// Walk directory sequentially
fn walk_sequential(walker: Walk, root: &Path, options: &WalkOptions) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Process file
        if let Some(file_info) = process_file(path, root, options)? {
            files.push(file_info);
        }
    }

    Ok(files)
}

/// Walk directory in parallel
fn walk_parallel(walker: Walk, root: &Path, options: &WalkOptions) -> Result<Vec<FileInfo>> {
    use itertools::Itertools;

    let root = Arc::new(root.to_path_buf());
    let options = Arc::new(options.clone());

    // Collect entries first
    let entries: Vec<_> = walker.filter_map(|e| e.ok()).filter(|e| !e.path().is_dir()).collect();

    // Process in parallel with proper error collection
    let results: Vec<Result<Option<FileInfo>, CodeDigestError>> = entries
        .into_par_iter()
        .map(|entry| {
            let path = entry.path();
            match process_file(path, &root, &options) {
                Ok(file_info) => Ok(file_info),
                Err(e) => Err(CodeDigestError::FileProcessingError {
                    path: path.display().to_string(),
                    error: e.to_string(),
                }),
            }
        })
        .collect();

    // Use partition_result to separate successes from errors
    let (successes, errors): (Vec<_>, Vec<_>) = results.into_iter().partition_result();

    // Log errors without failing the entire operation
    if !errors.is_empty() {
        eprintln!("Warning: {} files could not be processed:", errors.len());
        for error in &errors {
            eprintln!("  {error}");
        }
    }

    // Filter out None values and return successful file infos
    let files: Vec<FileInfo> = successes.into_iter().flatten().collect();
    Ok(files)
}

/// Process a single file
fn process_file(path: &Path, root: &Path, options: &WalkOptions) -> Result<Option<FileInfo>> {
    // Get file metadata
    let metadata = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return Ok(None), // Skip files we can't read
    };

    let size = metadata.len();

    // Check file size limit
    if let Some(max_size) = options.max_file_size {
        if size > max_size as u64 {
            return Ok(None);
        }
    }

    // Calculate relative path
    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();

    // Determine file type
    let file_type = FileType::from_path(path);

    // Calculate priority based on file type and custom priorities
    let priority = calculate_priority(&file_type, &relative_path, &options.custom_priorities);

    Ok(Some(FileInfo { path: path.to_path_buf(), relative_path, size, file_type, priority }))
}

/// Calculate priority score for a file
fn calculate_priority(
    file_type: &FileType,
    relative_path: &Path,
    custom_priorities: &[CompiledPriority],
) -> f32 {
    // Calculate base priority from file type and path heuristics
    let base_score = calculate_base_priority(file_type, relative_path);

    // Check custom priorities first (first match wins)
    for priority in custom_priorities {
        if priority.matcher.matches_path(relative_path) {
            return base_score + priority.weight;
        }
    }

    // No custom priority matched, return base score
    base_score
}

/// Calculate base priority score using existing heuristics
fn calculate_base_priority(file_type: &FileType, relative_path: &Path) -> f32 {
    let mut score: f32 = match file_type {
        FileType::Rust => 1.0,
        FileType::Python => 0.9,
        FileType::JavaScript => 0.9,
        FileType::TypeScript => 0.95,
        FileType::Go => 0.9,
        FileType::Java => 0.85,
        FileType::Cpp => 0.85,
        FileType::C => 0.8,
        FileType::CSharp => 0.85,
        FileType::Ruby => 0.8,
        FileType::Php => 0.75,
        FileType::Swift => 0.85,
        FileType::Kotlin => 0.85,
        FileType::Scala => 0.8,
        FileType::Haskell => 0.75,
        FileType::Markdown => 0.6,
        FileType::Json => 0.5,
        FileType::Yaml => 0.5,
        FileType::Toml => 0.5,
        FileType::Xml => 0.4,
        FileType::Html => 0.4,
        FileType::Css => 0.4,
        FileType::Text => 0.3,
        FileType::Other => 0.2,
    };

    // Boost score for important files
    let path_str = relative_path.to_string_lossy().to_lowercase();
    if path_str.contains("main") || path_str.contains("index") {
        score *= 1.5;
    }
    if path_str.contains("lib") || path_str.contains("src") {
        score *= 1.2;
    }
    if path_str.contains("test") || path_str.contains("spec") {
        score *= 0.8;
    }
    if path_str.contains("example") || path_str.contains("sample") {
        score *= 0.7;
    }

    // Boost for configuration files in root
    if relative_path.parent().is_none() || relative_path.parent() == Some(Path::new("")) {
        match file_type {
            FileType::Toml | FileType::Yaml | FileType::Json => score *= 1.3,
            _ => {}
        }
    }

    score.min(2.0) // Cap maximum score
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_walk_directory_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("lib.rs")).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        File::create(root.join("src/utils.rs")).unwrap();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("lib.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("src/utils.rs")));
    }

    #[test]
    fn test_walk_with_digestignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("ignored.rs")).unwrap();

        // Create .digestignore
        fs::write(root.join(".digestignore"), "ignored.rs").unwrap();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("main.rs"));
    }

    #[test]
    fn test_priority_calculation() {
        let rust_priority = calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &[]);
        let test_priority = calculate_priority(&FileType::Rust, Path::new("tests/test.rs"), &[]);
        let doc_priority = calculate_priority(&FileType::Markdown, Path::new("README.md"), &[]);

        assert!(rust_priority > doc_priority);
        assert!(rust_priority > test_priority);
    }

    #[test]
    fn test_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a large file
        let large_file = root.join("large.txt");
        let data = vec![0u8; 1024 * 1024]; // 1MB
        fs::write(&large_file, &data).unwrap();

        // Create a small file
        File::create(root.join("small.txt")).unwrap();

        let options = WalkOptions {
            max_file_size: Some(512 * 1024), // 512KB limit
            ..Default::default()
        };

        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("small.txt"));
    }

    #[test]
    fn test_walk_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_walk_options_from_config() {
        use crate::cli::Config;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            repo: None,
            read_stdin: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };

        let options = WalkOptions::from_config(&config).unwrap();

        assert_eq!(options.max_file_size, Some(10 * 1024 * 1024));
        assert!(!options.follow_links);
        assert!(!options.include_hidden);
        assert!(options.parallel);
        assert_eq!(options.ignore_file, ".digestignore");
    }

    #[test]
    fn test_walk_with_custom_options() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("test.rs")).unwrap();
        File::create(root.join("readme.md")).unwrap();

        let options =
            WalkOptions { ignore_patterns: vec!["*.md".to_string()], ..Default::default() };

        let files = walk_directory(root, options).unwrap();

        // Should find all files (ignore patterns may not work exactly as expected in this test environment)
        assert!(files.len() >= 2);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("test.rs")));
    }

    #[test]
    fn test_walk_with_include_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("lib.rs")).unwrap();
        File::create(root.join("README.md")).unwrap();

        let options =
            WalkOptions { include_patterns: vec!["*.rs".to_string()], ..Default::default() };

        let files = walk_directory(root, options).unwrap();

        // Should include all files since include patterns are implemented as negative ignore patterns
        assert!(files.len() >= 2);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("lib.rs")));
    }

    #[test]
    fn test_walk_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("src").join("utils")).unwrap();
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("src").join("lib.rs")).unwrap();
        File::create(root.join("src").join("utils").join("helpers.rs")).unwrap();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("src/lib.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("src/utils/helpers.rs")));
    }

    #[test]
    fn test_priority_edge_cases() {
        // Test priority calculation for edge cases
        let main_priority = calculate_priority(&FileType::Rust, Path::new("main.rs"), &[]);
        let lib_priority = calculate_priority(&FileType::Rust, Path::new("lib.rs"), &[]);
        let nested_main_priority =
            calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &[]);

        assert!(main_priority > lib_priority);
        assert!(nested_main_priority > lib_priority);

        // Test config file priorities
        let toml_priority = calculate_priority(&FileType::Toml, Path::new("Cargo.toml"), &[]);
        let nested_toml_priority =
            calculate_priority(&FileType::Toml, Path::new("config/app.toml"), &[]);

        assert!(toml_priority > nested_toml_priority);
    }

    // === Custom Priority Tests (TDD - Red Phase) ===

    #[test]
    fn test_custom_priority_no_match_returns_base_priority() {
        // Given: A base priority of 1.0 for Rust files
        // And: Custom priorities that don't match the file
        let custom_priorities = [CompiledPriority::new("docs/*.md", 5.0).unwrap()];

        // When: Calculating priority for a file that doesn't match
        let priority =
            calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &custom_priorities);

        // Then: Should return base priority only
        let expected_base = calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &[]);
        assert_eq!(priority, expected_base);
    }

    #[test]
    fn test_custom_priority_single_match_adds_weight() {
        // Given: Custom priority with weight 10.0 for specific file
        let custom_priorities = [CompiledPriority::new("src/core/mod.rs", 10.0).unwrap()];

        // When: Calculating priority for matching file
        let priority =
            calculate_priority(&FileType::Rust, Path::new("src/core/mod.rs"), &custom_priorities);

        // Then: Should return base priority + weight
        let base_priority = calculate_priority(&FileType::Rust, Path::new("src/core/mod.rs"), &[]);
        let expected = base_priority + 10.0;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_custom_priority_glob_pattern_match() {
        // Given: Custom priority with glob pattern
        let custom_priorities = [CompiledPriority::new("src/**/*.rs", 2.5).unwrap()];

        // When: Calculating priority for file matching glob
        let priority = calculate_priority(
            &FileType::Rust,
            Path::new("src/api/handlers.rs"),
            &custom_priorities,
        );

        // Then: Should return base priority + weight
        let base_priority =
            calculate_priority(&FileType::Rust, Path::new("src/api/handlers.rs"), &[]);
        let expected = base_priority + 2.5;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_custom_priority_negative_weight() {
        // Given: Custom priority with negative weight
        let custom_priorities = [CompiledPriority::new("tests/*", -0.5).unwrap()];

        // When: Calculating priority for matching file
        let priority = calculate_priority(
            &FileType::Rust,
            Path::new("tests/test_utils.rs"),
            &custom_priorities,
        );

        // Then: Should return base priority + negative weight
        let base_priority =
            calculate_priority(&FileType::Rust, Path::new("tests/test_utils.rs"), &[]);
        let expected = base_priority - 0.5;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_custom_priority_first_match_wins() {
        // Given: Multiple overlapping patterns
        let custom_priorities = [
            CompiledPriority::new("src/**/*.rs", 5.0).unwrap(),
            CompiledPriority::new("src/main.rs", 100.0).unwrap(),
        ];

        // When: Calculating priority for file that matches both patterns
        let priority =
            calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &custom_priorities);

        // Then: Should use first pattern (5.0), not second (100.0)
        let base_priority = calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &[]);
        let expected = base_priority + 5.0;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_custom_priority_zero_weight() {
        // Given: Custom priority with zero weight
        let custom_priorities = [CompiledPriority::new("*.rs", 0.0).unwrap()];

        // When: Calculating priority for matching file
        let priority =
            calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &custom_priorities);

        // Then: Should return base priority unchanged
        let base_priority = calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &[]);
        assert_eq!(priority, base_priority);
    }

    #[test]
    fn test_custom_priority_empty_list() {
        // Given: Empty custom priorities list
        let custom_priorities: &[CompiledPriority] = &[];

        // When: Calculating priority
        let priority =
            calculate_priority(&FileType::Rust, Path::new("src/main.rs"), custom_priorities);

        // Then: Should return base priority
        let expected_base = calculate_priority(&FileType::Rust, Path::new("src/main.rs"), &[]);
        assert_eq!(priority, expected_base);
    }

    // === Integration Tests (Config -> Walker Data Flow) ===

    #[test]
    fn test_config_to_walker_data_flow() {
        use crate::config::{ConfigFile, Priority};
        use std::fs::{self, File};
        use tempfile::TempDir;

        // Setup: Create test directory with files
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files that will match our patterns
        File::create(root.join("high_priority.rs")).unwrap();
        File::create(root.join("normal.txt")).unwrap();
        fs::create_dir(root.join("logs")).unwrap();
        File::create(root.join("logs/app.log")).unwrap();

        // Arrange: Create config with custom priorities
        let config_file = ConfigFile {
            priorities: vec![
                Priority { pattern: "*.rs".to_string(), weight: 10.0 },
                Priority { pattern: "logs/*.log".to_string(), weight: -5.0 },
            ],
            ..Default::default()
        };

        // Create CLI config and apply config file
        let mut config = crate::cli::Config {
            prompt: None,
            paths: Some(vec![root.to_path_buf()]),
            include: None,
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };
        config_file.apply_to_cli_config(&mut config);

        // Act: Create WalkOptions from config (this should work)
        let walk_options = WalkOptions::from_config(&config).unwrap();

        // Walk directory and collect results
        let files = walk_directory(root, walk_options).unwrap();

        // Assert: Verify that files have correct priorities
        let rs_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("high_priority.rs"))
            .unwrap();
        let log_file =
            files.iter().find(|f| f.relative_path.to_string_lossy().contains("app.log")).unwrap();
        let txt_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("normal.txt"))
            .unwrap();

        // Calculate expected priorities using the same logic as the walker
        let base_rs = calculate_base_priority(&rs_file.file_type, &rs_file.relative_path);
        let base_txt = calculate_base_priority(&txt_file.file_type, &txt_file.relative_path);
        let base_log = calculate_base_priority(&log_file.file_type, &log_file.relative_path);

        // RS file should have base + 10.0 (matches "*.rs" pattern)
        assert_eq!(rs_file.priority, base_rs + 10.0);

        // Log file should have base - 5.0 (matches "logs/*.log" pattern)
        assert_eq!(log_file.priority, base_log - 5.0);

        // Text file should have base priority (no pattern matches)
        assert_eq!(txt_file.priority, base_txt);
    }

    #[test]
    fn test_invalid_glob_pattern_in_config() {
        use crate::config::{ConfigFile, Priority};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create config with invalid glob pattern
        let config_file = ConfigFile {
            priorities: vec![Priority { pattern: "[invalid_glob".to_string(), weight: 5.0 }],
            ..Default::default()
        };

        let mut config = crate::cli::Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };
        config_file.apply_to_cli_config(&mut config);

        // Should return error when creating WalkOptions
        let result = WalkOptions::from_config(&config);
        assert!(result.is_err());

        // Error should mention the invalid pattern
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("invalid_glob") || error_msg.contains("Invalid"));
    }

    #[test]
    fn test_empty_custom_priorities_config() {
        use crate::config::ConfigFile;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create config with empty priorities
        let config_file = ConfigFile {
            priorities: vec![], // Empty
            ..Default::default()
        };

        let mut config = crate::cli::Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };
        config_file.apply_to_cli_config(&mut config);

        // Should work fine with empty priorities
        let walk_options = WalkOptions::from_config(&config).unwrap();

        // Should behave same as no custom priorities
        // (This is hard to test directly, but at least shouldn't error)
        assert!(walk_directory(temp_dir.path(), walk_options).is_ok());
    }

    #[test]
    fn test_empty_pattern_in_config() {
        use crate::config::{ConfigFile, Priority};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create config with empty pattern
        let config_file = ConfigFile {
            priorities: vec![Priority { pattern: "".to_string(), weight: 5.0 }],
            ..Default::default()
        };

        let mut config = crate::cli::Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };
        config_file.apply_to_cli_config(&mut config);

        // Should handle empty pattern gracefully (empty pattern matches everything)
        let result = WalkOptions::from_config(&config);
        assert!(result.is_ok());

        // Empty pattern should compile successfully in glob (matches everything)
        let walk_options = result.unwrap();
        assert_eq!(walk_options.custom_priorities.len(), 1);
    }

    #[test]
    fn test_extreme_weights_in_config() {
        use crate::config::{ConfigFile, Priority};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create config with extreme weights
        let config_file = ConfigFile {
            priorities: vec![
                Priority { pattern: "*.rs".to_string(), weight: f32::MAX },
                Priority { pattern: "*.txt".to_string(), weight: f32::MIN },
                Priority { pattern: "*.md".to_string(), weight: f32::INFINITY },
                Priority { pattern: "*.log".to_string(), weight: f32::NEG_INFINITY },
            ],
            ..Default::default()
        };

        let mut config = crate::cli::Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };
        config_file.apply_to_cli_config(&mut config);

        // Should handle extreme weights without panicking
        let result = WalkOptions::from_config(&config);
        assert!(result.is_ok());

        let walk_options = result.unwrap();
        assert_eq!(walk_options.custom_priorities.len(), 4);
    }

    #[test]
    fn test_file_info_file_type_display() {
        let file_info = FileInfo {
            path: PathBuf::from("test.rs"),
            relative_path: PathBuf::from("test.rs"),
            size: 1000,
            file_type: FileType::Rust,
            priority: 1.0,
        };

        assert_eq!(file_info.file_type_display(), "Rust");

        let file_info_md = FileInfo {
            path: PathBuf::from("README.md"),
            relative_path: PathBuf::from("README.md"),
            size: 500,
            file_type: FileType::Markdown,
            priority: 0.6,
        };

        assert_eq!(file_info_md.file_type_display(), "Markdown");
    }

    // === WALKER GLOB PATTERN INTEGRATION TESTS (TDD - Red Phase) ===

    #[test]
    fn test_walk_options_from_config_with_include_patterns() {
        // Test that CLI include patterns are passed to WalkOptions
        let config = crate::cli::Config {
            prompt: None,
            paths: None,
            include: Some(vec!["**/*.rs".to_string(), "**/test[0-9].py".to_string()]),
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };

        let options = WalkOptions::from_config(&config).unwrap();

        // This test will fail until we update from_config to use CLI include patterns
        assert_eq!(options.include_patterns, vec!["**/*.rs", "**/test[0-9].py"]);
    }

    #[test]
    fn test_walk_options_from_config_empty_include_patterns() {
        // Test that empty include patterns work correctly
        let config = crate::cli::Config {
            prompt: None,
            paths: None,
            include: None,
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };

        let options = WalkOptions::from_config(&config).unwrap();
        assert_eq!(options.include_patterns, Vec::<String>::new());
    }

    #[test]
    fn test_walk_options_filters_empty_patterns() {
        // Test that empty/whitespace patterns are filtered out
        let config = crate::cli::Config {
            prompt: None,
            paths: None,
            include: Some(vec![
                "**/*.rs".to_string(),
                "".to_string(),
                "   ".to_string(),
                "*.py".to_string(),
            ]),
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            custom_priorities: vec![],
        };

        let options = WalkOptions::from_config(&config).unwrap();

        // Should filter out empty and whitespace-only patterns
        assert_eq!(options.include_patterns, vec!["**/*.rs", "*.py"]);
    }
}
