# API Reference

Complete API reference for using context-creator programmatically and integrating it into other applications.

## Library Usage

### Basic Library Integration

```rust
use code_context::{Config, run};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // Create configuration
    let config = Config {
        directory: PathBuf::from("/path/to/project"),
        output_file: Some(PathBuf::from("output.md")),
        max_tokens: Some(50000),
        progress: true,
        verbose: false,
        quiet: false,
        llm_tool: code_context::cli::LlmTool::Gemini,
        config: None,
        prompt: None,
    };
    
    // Run context-creator
    run(config)?;
    
    Ok(())
}
```

### Advanced Configuration

```rust
use code_context::{
    Config,
    core::{
        context::{contextOptions, generate_markdown},
        walker::{WalkOptions, walk_directory},
        prioritizer::prioritize_files,
    },
};

fn advanced_usage() -> anyhow::Result<()> {
    let directory = PathBuf::from("/path/to/project");
    
    // Configure directory walking
    let walk_options = WalkOptions {
        follow_symlinks: false,
        ignore_patterns: vec![
            "target/".to_string(),
            "node_modules/".to_string(),
        ],
        include_patterns: vec![
            "src/**/*.rs".to_string(),
        ],
        max_depth: Some(10),
        max_file_size: Some(1024 * 1024), // 1MB
    };
    
    // Walk directory
    let files = walk_directory(&directory, walk_options)?;
    
    // Configure context generation
    let context_options = contextOptions {
        max_tokens: Some(50000),
        include_tree: true,
        include_stats: true,
        group_by_type: false,
        sort_by_priority: true,
        file_header_template: "## {path}".to_string(),
        doc_header_template: "# Code Analysis".to_string(),
        include_toc: true,
    };
    
    // Prioritize files based on token limits
    let prioritized_files = prioritize_files(files, &context_options)?;
    
    // Generate markdown
    let markdown = generate_markdown(prioritized_files, context_options)?;
    
    // Write output
    std::fs::write("analysis.md", markdown)?;
    
    Ok(())
}
```

## Core Types

### Config

Main configuration structure for context-creator.

```rust
pub struct Config {
    /// The prompt to send to the LLM
    pub prompt: Option<String>,
    
    /// The path to the directory to process
    pub directory: PathBuf,
    
    /// The path to the output Markdown file
    pub output_file: Option<PathBuf>,
    
    /// Maximum number of tokens for the generated context
    pub max_tokens: Option<usize>,
    
    /// LLM CLI tool to use for processing
    pub llm_tool: LlmTool,
    
    /// Suppress all output except for errors
    pub quiet: bool,
    
    /// Enable verbose logging
    pub verbose: bool,
    
    /// Path to configuration file
    pub config: Option<PathBuf>,
    
    /// Show progress indicators during processing
    pub progress: bool,
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), CodecontextError>;
    
    /// Load configuration from file if specified
    pub fn load_from_file(&mut self) -> Result<(), CodecontextError>;
}
```

### LlmTool

Enumeration of supported LLM CLI tools.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTool {
    /// Use gemini (default)
    Gemini,
    /// Use codex CLI
    Codex,
}

impl LlmTool {
    /// Get the command name for the tool
    pub fn command(&self) -> &'static str;
    
    /// Get the installation instructions for the tool
    pub fn install_instructions(&self) -> &'static str;
}
```

### WalkOptions

Configuration for directory traversal.

```rust
pub struct WalkOptions {
    /// Follow symbolic links
    pub follow_symlinks: bool,
    
    /// Patterns to ignore (glob format)
    pub ignore_patterns: Vec<String>,
    
    /// Patterns to include (overrides ignore)
    pub include_patterns: Vec<String>,
    
    /// Maximum directory depth
    pub max_depth: Option<usize>,
    
    /// Maximum file size to process (bytes)
    pub max_file_size: Option<u64>,
}

impl WalkOptions {
    /// Create WalkOptions from CLI config
    pub fn from_config(config: &Config) -> Result<Self>;
}

impl Default for WalkOptions {
    fn default() -> Self;
}
```

### contextOptions

Configuration for markdown generation.

```rust
pub struct contextOptions {
    /// Maximum tokens allowed in the output
    pub max_tokens: Option<usize>,
    
    /// Include file tree in output
    pub include_tree: bool,
    
    /// Include token count statistics
    pub include_stats: bool,
    
    /// Group files by type
    pub group_by_type: bool,
    
    /// Sort files by priority
    pub sort_by_priority: bool,
    
    /// Template for file headers
    pub file_header_template: String,
    
    /// Template for the document header
    pub doc_header_template: String,
    
    /// Include table of contents
    pub include_toc: bool,
}

impl contextOptions {
    /// Create contextOptions from CLI config
    pub fn from_config(config: &Config) -> Result<Self>;
}

impl Default for contextOptions {
    fn default() -> Self;
}
```

### FileInfo

Information about a discovered file.

```rust
pub struct FileInfo {
    /// Absolute path to the file
    pub path: PathBuf,
    
    /// Relative path from the root directory
    pub relative_path: PathBuf,
    
    /// File size in bytes
    pub size: u64,
    
    /// Detected file type
    pub file_type: FileType,
    
    /// Priority score for this file
    pub priority: f64,
}

impl FileInfo {
    /// Get display name for file type
    pub fn file_type_display(&self) -> &'static str;
}
```

### FileType

Enumeration of supported file types.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Cpp,
    C,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Markdown,
    Json,
    Yaml,
    Toml,
    Xml,
    Html,
    Css,
    Text,
    Other,
}
```

## Core Functions

### Directory Walking

```rust
/// Walk a directory and return information about discovered files
pub fn walk_directory(
    path: &Path,
    options: WalkOptions,
) -> Result<Vec<FileInfo>>;
```

**Example**:

```rust
use code_context::core::walker::{walk_directory, WalkOptions};
use std::path::Path;

let options = WalkOptions {
    ignore_patterns: vec!["target/".to_string()],
    ..Default::default()
};

let files = walk_directory(Path::new("/project"), options)?;
println!("Found {} files", files.len());
```

### File Prioritization

```rust
/// Prioritize files based on their importance and token limits
pub fn prioritize_files(
    files: Vec<FileInfo>,
    options: &contextOptions,
) -> Result<Vec<FileInfo>>;
```

**Example**:

```rust
use code_context::core::prioritizer::prioritize_files;
use code_context::core::context::contextOptions;

let options = contextOptions {
    max_tokens: Some(50000),
    sort_by_priority: true,
    ..Default::default()
};

let prioritized = prioritize_files(files, &options)?;
println!("Selected {} files after prioritization", prioritized.len());
```

### Markdown Generation

```rust
/// Generate markdown from a list of files
pub fn generate_markdown(
    files: Vec<FileInfo>,
    options: contextOptions,
) -> Result<String>;
```

**Example**:

```rust
use code_context::core::context::{generate_markdown, contextOptions};

let options = contextOptions {
    include_tree: true,
    include_stats: true,
    include_toc: true,
    file_header_template: "## {path}".to_string(),
    doc_header_template: "# Project Analysis".to_string(),
    ..Default::default()
};

let markdown = generate_markdown(files, options)?;
std::fs::write("output.md", markdown)?;
```

### Token Counting

```rust
use code_context::core::token::TokenCounter;

/// High-level token counting functions
pub fn count_tokens(text: &str) -> Result<usize>;
pub fn count_file_tokens(path: &Path) -> Result<usize>;

/// Token counter with caching
pub struct TokenCounter {
    pub fn new() -> Result<Self>;
    pub fn count_tokens(&self, text: &str) -> Result<usize>;
    pub fn count_file_tokens(&self, content: &str, path: &str) -> Result<FileTokenCount>;
    pub fn count_tokens_parallel(&self, texts: &[String]) -> Result<Vec<usize>>;
}
```

**Example**:

```rust
use code_context::core::token::{TokenCounter, count_tokens};

// Simple token counting
let count = count_tokens("Hello, world!")?;
println!("Tokens: {}", count);

// Advanced token counting with caching
let counter = TokenCounter::new()?;
let count = counter.count_tokens("println!(\"Hello, world!\");")?;
println!("Rust code tokens: {}", count);

// Parallel token counting
let texts = vec!["text1".to_string(), "text2".to_string()];
let counts = counter.count_tokens_parallel(&texts)?;
```

## Configuration API

### Configuration Loading

```rust
use code_context::config::{ConfigFile, Priority};

/// Load configuration from TOML file
let config = ConfigFile::load_from_file("config.toml")?;

/// Load from default locations
let config = ConfigFile::load_default()?;

/// Apply configuration to CLI config
config.apply_to_cli_config(&mut cli_config);
```

### Configuration Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub defaults: Defaults,
    
    #[serde(default)]
    pub priorities: Vec<Priority>,
    
    #[serde(default)]
    pub ignore: Vec<String>,
    
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub max_tokens: Option<usize>,
    pub progress: Option<bool>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    pub pattern: String,
    pub weight: f64,
    pub description: Option<String>,
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CodecontextError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("File processing error: {0}")]
    FileProcessing(String),
    
    #[error("Token counting error: {0}")]
    TokenCounting(String),
    
    #[error("LLM tool not found: {tool}. {install_instructions}")]
    LlmToolNotFound {
        tool: String,
        install_instructions: String,
    },
    
    #[error("Subprocess error: {0}")]
    SubprocessError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Error Handling Examples

```rust
use code_context::{run, Config, CodecontextError};

fn handle_errors() -> anyhow::Result<()> {
    let config = Config::default();
    
    match run(config) {
        Ok(()) => println!("Success!"),
        Err(e) => {
            match e.downcast_ref::<CodecontextError>() {
                Some(CodecontextError::LlmToolNotFound { tool, install_instructions }) => {
                    eprintln!("LLM tool '{}' not found.", tool);
                    eprintln!("Install instructions: {}", install_instructions);
                },
                Some(CodecontextError::InvalidPath(path)) => {
                    eprintln!("Invalid path: {}", path);
                },
                Some(CodecontextError::TokenCounting(msg)) => {
                    eprintln!("Token counting failed: {}", msg);
                },
                _ => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }
    
    Ok(())
}
```

## Utility Functions

### File Type Detection

```rust
use code_context::utils::file_ext::{detect_file_type, is_binary_file};

/// Detect file type from path
let file_type = detect_file_type(Path::new("src/main.rs"));
println!("File type: {:?}", file_type);

/// Check if file is binary
let is_binary = is_binary_file(Path::new("image.png"));
println!("Is binary: {}", is_binary);
```

### Pattern Matching

```rust
use code_context::utils::patterns::{matches_pattern, compile_patterns};

/// Test if path matches glob pattern
let matches = matches_pattern("src/main.rs", "src/**/*.rs");
println!("Matches: {}", matches);

/// Compile multiple patterns for efficient matching
let patterns = compile_patterns(&["*.rs", "*.toml"])?;
let matches = patterns.is_match("Cargo.toml");
```

## Integration Examples

### Custom CLI Tool

```rust
use clap::{Parser, Subcommand};
use code_context::{Config, run};

#[derive(Parser)]
#[command(name = "my-analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Analyze {
        #[arg(short, long)]
        directory: PathBuf,
        
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        #[arg(long)]
        max_tokens: Option<usize>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Analyze { directory, output, max_tokens } => {
            let config = Config {
                directory,
                output_file: output,
                max_tokens,
                progress: true,
                verbose: false,
                quiet: false,
                llm_tool: code_context::cli::LlmTool::Gemini,
                config: None,
                prompt: None,
            };
            
            run(config)?;
        }
    }
    
    Ok(())
}
```

### Web Service Integration

```rust
use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use code_context::{Config, run};

#[derive(Deserialize)]
struct AnalyzeRequest {
    directory: String,
    max_tokens: Option<usize>,
}

#[derive(Serialize)]
struct AnalyzeResponse {
    success: bool,
    markdown: Option<String>,
    error: Option<String>,
}

async fn analyze_endpoint(Query(params): Query<AnalyzeRequest>) -> Json<AnalyzeResponse> {
    let config = Config {
        directory: params.directory.into(),
        output_file: None,
        max_tokens: params.max_tokens,
        progress: false,
        verbose: false,
        quiet: true,
        llm_tool: code_context::cli::LlmTool::Gemini,
        config: None,
        prompt: None,
    };
    
    match run_and_capture_output(config) {
        Ok(markdown) => Json(AnalyzeResponse {
            success: true,
            markdown: Some(markdown),
            error: None,
        }),
        Err(e) => Json(AnalyzeResponse {
            success: false,
            markdown: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/analyze", get(analyze_endpoint));
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Async Integration

```rust
use tokio::task;
use code_context::{Config, run};

async fn async_analysis(directory: PathBuf) -> anyhow::Result<String> {
    // Run context-creator in a blocking task
    let config = Config {
        directory,
        output_file: None,
        max_tokens: Some(50000),
        progress: false,
        verbose: false,
        quiet: true,
        llm_tool: code_context::cli::LlmTool::Gemini,
        config: None,
        prompt: None,
    };
    
    let markdown = task::spawn_blocking(move || {
        // Capture output instead of writing to file
        run_and_capture_output(config)
    }).await??;
    
    Ok(markdown)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let directories = vec![
        PathBuf::from("/project1"),
        PathBuf::from("/project2"),
        PathBuf::from("/project3"),
    ];
    
    // Process multiple directories concurrently
    let tasks: Vec<_> = directories.into_iter()
        .map(|dir| async_analysis(dir))
        .collect();
    
    let results = futures::future::try_join_all(tasks).await?;
    
    for (i, markdown) in results.iter().enumerate() {
        println!("Project {} analysis: {} characters", i + 1, markdown.len());
    }
    
    Ok(())
}
```

## Testing Integration

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_directory_analysis() -> anyhow::Result<()> {
        // Create temporary project
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().join("test_project");
        fs::create_dir_all(&project_dir)?;
        
        // Create test files
        fs::write(project_dir.join("main.rs"), r#"
            fn main() {
                println!("Hello, world!");
            }
        "#)?;
        
        fs::write(project_dir.join("lib.rs"), r#"
            pub fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#)?;
        
        // Run analysis
        let config = Config {
            directory: project_dir,
            output_file: None,
            max_tokens: Some(10000),
            progress: false,
            verbose: false,
            quiet: true,
            llm_tool: code_context::cli::LlmTool::Gemini,
            config: None,
            prompt: None,
        };
        
        let markdown = run_and_capture_output(config)?;
        
        // Verify output
        assert!(markdown.contains("# Code context"));
        assert!(markdown.contains("main.rs"));
        assert!(markdown.contains("lib.rs"));
        assert!(markdown.contains("Hello, world!"));
        
        Ok(())
    }
}
```

### Benchmark Testing

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use code_context::{Config, run_and_capture_output};

fn benchmark_analysis(c: &mut Criterion) {
    let config = Config {
        directory: PathBuf::from("test-project"),
        output_file: None,
        max_tokens: Some(50000),
        progress: false,
        verbose: false,
        quiet: true,
        llm_tool: code_context::cli::LlmTool::Gemini,
        config: None,
        prompt: None,
    };
    
    c.bench_function("project analysis", |b| {
        b.iter(|| {
            black_box(run_and_capture_output(black_box(config.clone())).unwrap());
        });
    });
}

criterion_group!(benches, benchmark_analysis);
criterion_main!(benches);
```

## Performance Considerations

### Memory Management

```rust
use code_context::core::walker::WalkOptions;

// For large projects, use streaming approach
let walk_options = WalkOptions {
    max_file_size: Some(1024 * 1024), // 1MB limit per file
    ..Default::default()
};

// Process files in chunks
const CHUNK_SIZE: usize = 100;
for chunk in files.chunks(CHUNK_SIZE) {
    let markdown = generate_markdown(chunk.to_vec(), options.clone())?;
    // Process chunk...
}
```

### Parallel Processing

```rust
use rayon::prelude::*;
use code_context::core::token::TokenCounter;

// Parallel token counting
let counter = TokenCounter::new()?;
let token_counts: Vec<usize> = files
    .par_iter()
    .map(|file| {
        let content = std::fs::read_to_string(&file.path)?;
        counter.count_tokens(&content)
    })
    .collect::<Result<Vec<_>, _>>()?;
```

### Caching

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Simple caching example
type Cache = Arc<Mutex<HashMap<PathBuf, String>>>;

fn cached_analysis(path: &Path, cache: Cache) -> anyhow::Result<String> {
    // Check cache first
    if let Ok(cache_guard) = cache.lock() {
        if let Some(cached) = cache_guard.get(path) {
            return Ok(cached.clone());
        }
    }
    
    // Generate analysis
    let config = Config {
        directory: path.to_path_buf(),
        // ... other config
    };
    
    let result = run_and_capture_output(config)?;
    
    // Store in cache
    if let Ok(mut cache_guard) = cache.lock() {
        cache_guard.insert(path.to_path_buf(), result.clone());
    }
    
    Ok(result)
}
```

## Logging Module

### Structured Logging with Tracing

The logging module provides structured logging capabilities using the `tracing` and `tracing-subscriber` crates.

```rust
use context_creator::logging::{init_logging, get_log_level};
use context_creator::Config;
use tracing::{info, debug, warn};

fn main() -> anyhow::Result<()> {
    // Initialize logging based on configuration
    let config = Config::default();
    init_logging(&config)?;
    
    // Log at different levels
    info!("Starting application");
    debug!("Debug information");
    warn!("Warning message");
    
    Ok(())
}
```

### Log Level Configuration

```rust
use context_creator::logging::get_log_level;
use tracing::Level;

// Get log level based on verbosity and progress flags
let level = get_log_level(1, false, false); // Returns Level::DEBUG
let level = get_log_level(2, false, false); // Returns Level::TRACE
let level = get_log_level(0, true, false);  // Returns Level::ERROR (quiet mode)
let level = get_log_level(0, false, true);  // Returns Level::INFO (progress mode)
```

### JSON Logging

When the `--log-format json` option is used, logs are output in structured JSON format:

```rust
use context_creator::cli::LogFormat;

let mut config = Config::default();
config.log_format = LogFormat::Json;
init_logging(&config)?;

// Logs will be output as JSON:
// {"timestamp":"2024-01-01T12:00:00Z","level":"INFO","message":"Starting"}
```

### Environment Variable Control

The logging system respects the `RUST_LOG` environment variable for fine-grained control:

```bash
# Enable debug logging for specific modules
RUST_LOG=context_creator::walker=debug

# Enable trace logging for all modules
RUST_LOG=trace

# Multiple module configuration
RUST_LOG=context_creator::semantic=trace,context_creator::walker=debug
```

This API reference provides comprehensive documentation for integrating context-creator into other applications and using it programmatically.