//! RPC method handlers for the MCP server

use super::{
    CodebaseRpcServer, HealthResponse, HealthRpcServer, ProcessLocalRequest, ProcessLocalResponse,
    ProcessRemoteRequest, ProcessRemoteResponse,
};
use anyhow::Result;
use jsonrpsee::core::RpcResult;
use std::path::Path;
use std::time::{Instant, SystemTime};

/// Implementation of health check RPC methods
pub struct HealthRpcImpl;

#[jsonrpsee::core::async_trait]
impl HealthRpcServer for HealthRpcImpl {
    async fn health_check(&self) -> RpcResult<HealthResponse> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32000, "Timestamp error", Some(e.to_string()))
            })?
            .as_secs();

        Ok(HealthResponse {
            status: "healthy".to_string(),
            timestamp,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

/// Implementation of codebase processing RPC handlers
pub struct CodebaseRpcImpl {
    cache: std::sync::Arc<crate::mcp_server::cache::McpCache>,
}

impl CodebaseRpcImpl {
    pub fn new(cache: std::sync::Arc<crate::mcp_server::cache::McpCache>) -> Self {
        Self { cache }
    }
}

#[jsonrpsee::core::async_trait]
impl CodebaseRpcServer for CodebaseRpcImpl {
    async fn process_local_codebase(
        &self,
        request: ProcessLocalRequest,
    ) -> RpcResult<ProcessLocalResponse> {
        let start = Instant::now();

        // Validate path security
        validate_path(&request.path)?;

        // Check cache first
        let cache_key = crate::mcp_server::cache::ProcessLocalCacheKey::from_request(&request);
        if let Some(cached) = self.cache.get_process_local(&cache_key).await {
            let processing_time_ms = start.elapsed().as_millis() as u64;
            return Ok(ProcessLocalResponse {
                answer: cached.answer,
                context: if request.include_context.unwrap_or(false) {
                    Some(cached.markdown)
                } else {
                    None
                },
                file_count: cached.file_count,
                token_count: cached.token_count,
                processing_time_ms,
                llm_tool: cached.llm_tool,
            });
        }

        // Use blocking task for file I/O
        let cache = self.cache.clone();
        let response = tokio::task::spawn_blocking(move || process_codebase_sync(request, start))
            .await
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Internal error", Some(e.to_string()))
            })?
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    -32603,
                    "Processing error",
                    Some(e.to_string()),
                )
            })?;

        // Cache the response
        let cache_value = crate::mcp_server::cache::ProcessLocalCacheValue {
            answer: response.answer.clone(),
            markdown: response.context.clone().unwrap_or_default(),
            file_count: response.file_count,
            token_count: response.token_count,
            llm_tool: response.llm_tool.clone(),
        };
        cache.set_process_local(cache_key, cache_value).await;

        Ok(response)
    }

    async fn process_remote_repo(
        &self,
        request: ProcessRemoteRequest,
    ) -> RpcResult<ProcessRemoteResponse> {
        let start = Instant::now();

        // Validate URL
        validate_url(&request.repo_url)?;

        // Clone the repository and process it
        let response = tokio::task::spawn_blocking(move || process_remote_sync(request, start))
            .await
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Internal error", Some(e.to_string()))
            })?
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    -32603,
                    "Processing error",
                    Some(e.to_string()),
                )
            })?;

        Ok(response)
    }

    async fn get_file_metadata(
        &self,
        request: super::GetFileMetadataRequest,
    ) -> RpcResult<super::GetFileMetadataResponse> {
        // Validate path security
        validate_path(&request.file_path)?;

        // Use blocking task for file I/O
        let response = tokio::task::spawn_blocking(move || get_file_metadata_sync(request))
            .await
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Internal error", Some(e.to_string()))
            })?
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    -32603,
                    "Processing error",
                    Some(e.to_string()),
                )
            })?;

        Ok(response)
    }

    async fn search_codebase(
        &self,
        request: super::SearchCodebaseRequest,
    ) -> RpcResult<super::SearchCodebaseResponse> {
        use std::time::Instant;
        let start = Instant::now();

        // Validate path security
        validate_path(&request.path)?;

        // Use blocking task for file I/O
        let response = tokio::task::spawn_blocking(move || search_codebase_sync(request, start))
            .await
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Internal error", Some(e.to_string()))
            })?
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Search error", Some(e.to_string()))
            })?;

        Ok(response)
    }

    async fn diff_files(
        &self,
        request: super::DiffFilesRequest,
    ) -> RpcResult<super::DiffFilesResponse> {
        // Validate paths security
        validate_path(&request.file1_path)?;
        validate_path(&request.file2_path)?;

        // Use blocking task for file I/O
        let response = tokio::task::spawn_blocking(move || diff_files_sync(request))
            .await
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Internal error", Some(e.to_string()))
            })?
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Diff error", Some(e.to_string()))
            })?;

        Ok(response)
    }

    async fn semantic_search(
        &self,
        request: super::SemanticSearchRequest,
    ) -> RpcResult<super::SemanticSearchResponse> {
        use std::time::Instant;
        let start = Instant::now();

        // Validate path security
        validate_path(&request.path)?;

        // Use blocking task for file I/O and AST parsing
        let response = tokio::task::spawn_blocking(move || semantic_search_sync(request, start))
            .await
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32603, "Internal error", Some(e.to_string()))
            })?
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    -32603,
                    "Semantic search error",
                    Some(e.to_string()),
                )
            })?;

        Ok(response)
    }
}

/// Validate path for security issues
fn validate_path(path: &Path) -> RpcResult<()> {
    // Check for path traversal attempts
    let path_str = path.to_string_lossy();
    if path_str.contains("..") || path_str.contains('~') {
        return Err(jsonrpsee::types::ErrorObject::owned(
            -32602,
            "Invalid path: potential security risk",
            None::<String>,
        ));
    }

    // Ensure path is absolute or relative to current directory
    if !path.is_absolute() && !path.starts_with(".") && !path.exists() {
        return Err(jsonrpsee::types::ErrorObject::owned(
            -32602,
            "Invalid path: must be absolute or relative to current directory",
            None::<String>,
        ));
    }

    Ok(())
}

/// Validate URL for remote repositories
fn validate_url(url: &str) -> RpcResult<()> {
    // Basic URL validation
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(jsonrpsee::types::ErrorObject::owned(
            -32602,
            "Invalid URL: must start with http:// or https://",
            None::<String>,
        ));
    }

    // GitHub URL validation
    if url.contains("github.com") && !url.contains("/") {
        return Err(jsonrpsee::types::ErrorObject::owned(
            -32602,
            "Invalid GitHub URL format",
            None::<String>,
        ));
    }

    Ok(())
}

/// Synchronous implementation of codebase processing
fn process_codebase_sync(
    request: ProcessLocalRequest,
    start: Instant,
) -> Result<ProcessLocalResponse> {
    use crate::cli::{Config, LlmTool};
    use crate::core::cache::FileCache;
    use crate::core::context_builder::{generate_markdown, ContextOptions};
    use crate::core::prioritizer::prioritize_files;
    use crate::core::token::TokenCounter;
    use crate::core::walker::{walk_directory, WalkOptions};
    use std::sync::Arc;

    // Determine LLM tool
    let llm_tool = if let Some(tool_str) = &request.llm_tool {
        match tool_str.as_str() {
            "gemini" => LlmTool::Gemini,
            "codex" => LlmTool::Codex,
            _ => LlmTool::Gemini,
        }
    } else {
        LlmTool::Gemini
    };

    // Calculate effective token limit considering prompt
    let effective_max_tokens = if let Some(max_tokens) = request.max_tokens {
        max_tokens as usize
    } else {
        // Use LLM default if not specified
        llm_tool.default_max_tokens()
    };

    // Reserve tokens for prompt and response
    let prompt_tokens = if let Ok(counter) = TokenCounter::new() {
        counter
            .count_tokens(&request.prompt)
            .unwrap_or(request.prompt.len() / 4)
    } else {
        request.prompt.len() / 4 // Rough estimate
    };

    let safety_buffer = 1000; // For LLM response
    let context_tokens = effective_max_tokens.saturating_sub(prompt_tokens + safety_buffer);

    // Create a Config from the request
    let config = Config {
        paths: Some(vec![request.path.clone()]),
        include: if request.include_patterns.is_empty() {
            None
        } else {
            Some(request.include_patterns.clone())
        },
        ignore: if request.ignore_patterns.is_empty() {
            None
        } else {
            Some(request.ignore_patterns.clone())
        },
        trace_imports: request.include_imports,
        max_tokens: Some(context_tokens),
        llm_tool,
        // Enable prompt for proper context calculation
        prompt: Some(request.prompt.clone()),
        // Disable other options
        output_file: None,
        copy: false,
        verbose: 0,
        quiet: true,
        ..Default::default()
    };

    // Create walker options
    let walk_options = WalkOptions::from_config(&config)?;

    // Create context options
    let context_options = ContextOptions::from_config(&config)?;

    // Create file cache
    let cache = Arc::new(FileCache::new());

    // Walk the directory
    let files = walk_directory(&request.path, walk_options)?;

    // Prioritize files if max tokens is set
    let prioritized_files = if context_options.max_tokens.is_some() {
        prioritize_files(files, &context_options, cache.clone())?
    } else {
        files
    };

    // Generate markdown
    let output = generate_markdown(prioritized_files, context_options, cache)?;

    // Count tokens
    let token_counter = crate::core::token::TokenCounter::new()?;
    let token_count = token_counter.count_tokens(&output)?;

    // Count files (count markdown headers starting with ##)
    let file_count = output
        .lines()
        .filter(|line| {
            line.starts_with("## ")
                && !line.starts_with("## Table of")
                && !line.starts_with("## Statistics")
                && !line.starts_with("## File Structure")
        })
        .count();

    // Execute LLM with prompt and context
    let answer = execute_llm_sync(&request.prompt, &output, request.llm_tool.as_deref())?;

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(ProcessLocalResponse {
        answer,
        context: if request.include_context.unwrap_or(false) {
            Some(output)
        } else {
            None
        },
        file_count,
        token_count,
        processing_time_ms,
        llm_tool: request.llm_tool.unwrap_or_else(|| "gemini".to_string()),
    })
}

/// Synchronous implementation of remote repository processing
fn process_remote_sync(
    request: ProcessRemoteRequest,
    start: Instant,
) -> Result<ProcessRemoteResponse> {
    use crate::cli::Config;
    use crate::core::cache::FileCache;
    use crate::core::context_builder::{generate_markdown, ContextOptions};
    use crate::core::prioritizer::prioritize_files;
    use crate::core::walker::{walk_directory, WalkOptions};
    use crate::remote;
    use std::sync::Arc;

    // Clone the repository
    let temp_dir = remote::fetch_repository(&request.repo_url, false)?;
    let repo_path = remote::get_repo_path(&temp_dir, &request.repo_url)?;

    // Extract repo name from URL
    let repo_name = request
        .repo_url
        .split('/')
        .next_back()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string();

    // Determine LLM tool
    let llm_tool = if let Some(tool_str) = &request.llm_tool {
        match tool_str.as_str() {
            "gemini" => crate::cli::LlmTool::Gemini,
            "codex" => crate::cli::LlmTool::Codex,
            _ => crate::cli::LlmTool::Gemini,
        }
    } else {
        crate::cli::LlmTool::Gemini
    };

    // Calculate effective token limit considering prompt
    let effective_max_tokens = if let Some(max_tokens) = request.max_tokens {
        max_tokens as usize
    } else {
        llm_tool.default_max_tokens()
    };

    // Reserve tokens for prompt and response
    let prompt_tokens = if let Ok(counter) = crate::core::token::TokenCounter::new() {
        counter
            .count_tokens(&request.prompt)
            .unwrap_or(request.prompt.len() / 4)
    } else {
        request.prompt.len() / 4
    };

    let safety_buffer = 1000;
    let context_tokens = effective_max_tokens.saturating_sub(prompt_tokens + safety_buffer);

    // Create a Config from the request
    let config = Config {
        paths: Some(vec![repo_path.clone()]),
        include: if request.include_patterns.is_empty() {
            None
        } else {
            Some(request.include_patterns.clone())
        },
        ignore: if request.ignore_patterns.is_empty() {
            None
        } else {
            Some(request.ignore_patterns.clone())
        },
        trace_imports: request.include_imports,
        max_tokens: Some(context_tokens),
        llm_tool,
        prompt: Some(request.prompt.clone()),
        // Disable other options
        output_file: None,
        copy: false,
        verbose: 0,
        quiet: true,
        ..Default::default()
    };

    // Create walker options
    let walk_options = WalkOptions::from_config(&config)?;

    // Create context options
    let context_options = ContextOptions::from_config(&config)?;

    // Create file cache
    let cache = Arc::new(FileCache::new());

    // Walk the directory
    let files = walk_directory(&repo_path, walk_options)?;

    // Prioritize files if max tokens is set
    let prioritized_files = if context_options.max_tokens.is_some() {
        prioritize_files(files, &context_options, cache.clone())?
    } else {
        files
    };

    // Generate markdown
    let output = generate_markdown(prioritized_files, context_options, cache)?;

    // Count tokens
    let token_counter = crate::core::token::TokenCounter::new()?;
    let token_count = token_counter.count_tokens(&output)?;

    // Count files (count markdown headers starting with ##)
    let file_count = output
        .lines()
        .filter(|line| {
            line.starts_with("## ")
                && !line.starts_with("## Table of")
                && !line.starts_with("## Statistics")
                && !line.starts_with("## File Structure")
        })
        .count();

    // Execute LLM with prompt and context
    let answer = execute_llm_sync(&request.prompt, &output, request.llm_tool.as_deref())?;

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(ProcessRemoteResponse {
        answer,
        context: if request.include_context.unwrap_or(false) {
            Some(output)
        } else {
            None
        },
        file_count,
        token_count,
        processing_time_ms,
        repo_name,
        llm_tool: request.llm_tool.unwrap_or_else(|| "gemini".to_string()),
    })
}

/// Synchronous implementation of file metadata retrieval
fn get_file_metadata_sync(
    request: super::GetFileMetadataRequest,
) -> Result<super::GetFileMetadataResponse> {
    use std::fs;
    use std::time::SystemTime;

    // Check if file exists
    if !request.file_path.exists() {
        return Err(anyhow::anyhow!(
            "File not found: {}",
            request.file_path.display()
        ));
    }

    // Get file metadata
    let metadata = fs::metadata(&request.file_path)?;

    // Get modification time as Unix timestamp
    let modified = metadata
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    // Check if it's a symlink
    let symlink_metadata = fs::symlink_metadata(&request.file_path)?;
    let is_symlink = symlink_metadata.is_symlink();

    // Determine language from file extension
    let language = request
        .file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext {
            "rs" => Some("rust"),
            "py" => Some("python"),
            "js" | "jsx" => Some("javascript"),
            "ts" | "tsx" => Some("typescript"),
            "go" => Some("go"),
            "java" => Some("java"),
            "c" => Some("c"),
            "cpp" | "cc" | "cxx" => Some("cpp"),
            "h" | "hpp" => Some("cpp"),
            "cs" => Some("csharp"),
            "rb" => Some("ruby"),
            "php" => Some("php"),
            "swift" => Some("swift"),
            "kt" | "kts" => Some("kotlin"),
            "scala" => Some("scala"),
            "r" => Some("r"),
            "lua" => Some("lua"),
            "dart" => Some("dart"),
            "jl" => Some("julia"),
            "hs" => Some("haskell"),
            "elm" => Some("elm"),
            "clj" | "cljs" => Some("clojure"),
            "ex" | "exs" => Some("elixir"),
            "ml" | "mli" => Some("ocaml"),
            "nim" => Some("nim"),
            "zig" => Some("zig"),
            _ => None,
        })
        .map(String::from);

    Ok(super::GetFileMetadataResponse {
        path: request.file_path,
        size: metadata.len(),
        modified,
        is_symlink,
        language,
    })
}

/// Synchronous implementation of codebase search
fn search_codebase_sync(
    request: super::SearchCodebaseRequest,
    start: std::time::Instant,
) -> Result<super::SearchCodebaseResponse> {
    use std::fs;
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;

    let mut results = Vec::new();
    let mut total_matches = 0;
    let mut files_searched = 0;

    // Create walker with optional file pattern
    let walker = WalkDir::new(&request.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in walker {
        let path = entry.path();

        // Apply file pattern filter if specified
        if let Some(ref pattern) = request.file_pattern {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Simple glob pattern matching
            if pattern.starts_with("*.") {
                let extension = pattern.trim_start_matches("*.");
                if !file_name.ends_with(extension) {
                    continue;
                }
            } else if !file_name.contains(pattern) {
                continue;
            }
        }

        // Skip binary files
        if is_binary_file(path) {
            continue;
        }

        files_searched += 1;

        // Search file content
        if let Ok(file) = fs::File::open(path) {
            let reader = BufReader::new(file);

            for (line_number, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if line.to_lowercase().contains(&request.query.to_lowercase()) {
                        total_matches += 1;

                        // Create search result
                        let result = super::SearchResult {
                            file_path: path.to_path_buf(),
                            line_number: line_number + 1, // 1-based line numbers
                            line_content: line.clone(),
                            match_context: get_match_context(&line, &request.query),
                        };

                        results.push(result);

                        // Check max results limit
                        if let Some(max) = request.max_results {
                            if results.len() >= max as usize {
                                break;
                            }
                        }
                    }
                }
            }

            // Stop searching if we've hit the max results
            if let Some(max) = request.max_results {
                if results.len() >= max as usize {
                    break;
                }
            }
        }
    }

    let search_time_ms = start.elapsed().as_millis() as u64;

    Ok(super::SearchCodebaseResponse {
        results,
        total_matches,
        files_searched,
        search_time_ms,
    })
}

/// Check if a file is likely binary
fn is_binary_file(path: &Path) -> bool {
    // Check common binary extensions
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if matches!(
            ext,
            "exe"
                | "dll"
                | "so"
                | "dylib"
                | "bin"
                | "obj"
                | "o"
                | "jpg"
                | "jpeg"
                | "png"
                | "gif"
                | "bmp"
                | "ico"
                | "webp"
                | "mp3"
                | "mp4"
                | "avi"
                | "mov"
                | "wav"
                | "flac"
                | "zip"
                | "tar"
                | "gz"
                | "bz2"
                | "7z"
                | "rar"
                | "pdf"
                | "doc"
                | "docx"
                | "xls"
                | "xlsx"
                | "ppt"
                | "pptx"
        ) {
            return true;
        }
    }

    // Check by reading first few bytes
    if let Ok(mut file) = std::fs::File::open(path) {
        use std::io::Read;
        let mut buffer = [0; 512];
        if let Ok(n) = file.read(&mut buffer) {
            // Check for null bytes (common in binary files)
            return buffer[..n].contains(&0);
        }
    }

    false
}

/// Get context around the match
fn get_match_context(line: &str, query: &str) -> String {
    if let Some(pos) = line.to_lowercase().find(&query.to_lowercase()) {
        let start = pos.saturating_sub(20);
        let end = (pos + query.len() + 20).min(line.len());

        let mut context = String::new();
        if start > 0 {
            context.push_str("...");
        }
        context.push_str(&line[start..end]);
        if end < line.len() {
            context.push_str("...");
        }
        context
    } else {
        line.to_string()
    }
}

/// Synchronous implementation of file diff
fn diff_files_sync(request: super::DiffFilesRequest) -> Result<super::DiffFilesResponse> {
    use std::fs;

    // Check if files exist
    if !request.file1_path.exists() {
        return Err(anyhow::anyhow!(
            "File not found: {}",
            request.file1_path.display()
        ));
    }
    if !request.file2_path.exists() {
        return Err(anyhow::anyhow!(
            "File not found: {}",
            request.file2_path.display()
        ));
    }

    // Read files
    let content1 = fs::read(&request.file1_path)?;
    let content2 = fs::read(&request.file2_path)?;

    // Check if either file is binary
    let is_binary = content1.contains(&0) || content2.contains(&0);

    if is_binary {
        return Ok(super::DiffFilesResponse {
            file1_path: request.file1_path,
            file2_path: request.file2_path,
            hunks: vec![],
            added_lines: 0,
            removed_lines: 0,
            is_binary: true,
        });
    }

    // Convert to strings for text diff
    let text1 = String::from_utf8_lossy(&content1);
    let text2 = String::from_utf8_lossy(&content2);

    // Split into lines
    let lines1: Vec<&str> = text1.lines().collect();
    let lines2: Vec<&str> = text2.lines().collect();

    // Perform simple line-by-line diff
    let hunks = compute_diff(&lines1, &lines2, request.context_lines.unwrap_or(3));

    // Count added and removed lines
    let mut added_lines = 0;
    let mut removed_lines = 0;
    for hunk in &hunks {
        for line in hunk.content.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                added_lines += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                removed_lines += 1;
            }
        }
    }

    Ok(super::DiffFilesResponse {
        file1_path: request.file1_path,
        file2_path: request.file2_path,
        hunks,
        added_lines,
        removed_lines,
        is_binary: false,
    })
}

/// Compute diff between two sets of lines
fn compute_diff(lines1: &[&str], lines2: &[&str], context_lines: u32) -> Vec<super::DiffHunk> {
    use std::cmp::min;

    let mut hunks = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < lines1.len() || j < lines2.len() {
        // Find next difference
        while i < lines1.len() && j < lines2.len() && lines1[i] == lines2[j] {
            i += 1;
            j += 1;
        }

        if i >= lines1.len() && j >= lines2.len() {
            break;
        }

        // Start of a hunk
        let hunk_start_i = i.saturating_sub(context_lines as usize);
        let hunk_start_j = j.saturating_sub(context_lines as usize);
        let mut hunk_content = String::new();

        // Add context before
        for k in hunk_start_i..i {
            if k < lines1.len() {
                hunk_content.push_str(&format!(" {}\n", lines1[k]));
            }
        }

        // Find end of differences
        let mut end_i = i;
        let mut end_j = j;

        // Simple algorithm: advance until we find matching lines again
        while end_i < lines1.len() || end_j < lines2.len() {
            let mut found_match = false;

            // Check if we can find a match by advancing either side
            if end_i < lines1.len() && end_j < lines2.len() && lines1[end_i] == lines2[end_j] {
                found_match = true;
            }

            if found_match {
                // Check if we have enough context lines matching
                let mut k = 0;
                while k < context_lines as usize
                    && end_i + k < lines1.len()
                    && end_j + k < lines2.len()
                    && lines1[end_i + k] == lines2[end_j + k]
                {
                    k += 1;
                }

                if k >= context_lines as usize {
                    break;
                }
            }

            // Add removed lines
            if end_i < lines1.len()
                && (end_j >= lines2.len()
                    || (end_i < lines1.len()
                        && end_j < lines2.len()
                        && lines1[end_i] != lines2[end_j]))
            {
                hunk_content.push_str(&format!("-{}\n", lines1[end_i]));
                end_i += 1;
            }

            // Add added lines
            if end_j < lines2.len()
                && (end_i >= lines1.len()
                    || (end_i < lines1.len()
                        && end_j < lines2.len()
                        && lines1[end_i - 1] != lines2[end_j]))
            {
                hunk_content.push_str(&format!("+{}\n", lines2[end_j]));
                end_j += 1;
            }
        }

        // Add context after
        let context_end_i = min(end_i + context_lines as usize, lines1.len());
        let context_end_j = min(end_j + context_lines as usize, lines2.len());

        for k in end_i..context_end_i {
            if k < lines1.len() && k - end_i < context_end_j - end_j {
                hunk_content.push_str(&format!(" {}\n", lines1[k]));
            }
        }

        // Create hunk
        let hunk = super::DiffHunk {
            old_start: hunk_start_i + 1, // 1-based
            old_lines: end_i - hunk_start_i,
            new_start: hunk_start_j + 1, // 1-based
            new_lines: end_j - hunk_start_j,
            content: hunk_content,
        };

        hunks.push(hunk);

        // Move to next position
        i = end_i;
        j = end_j;
    }

    hunks
}

/// Synchronous implementation of semantic search
fn semantic_search_sync(
    request: super::SemanticSearchRequest,
    start: std::time::Instant,
) -> Result<super::SemanticSearchResponse> {
    use crate::core::semantic::{get_analyzer_for_file, SemanticContext};
    use std::fs;
    use walkdir::WalkDir;

    let mut results = Vec::new();
    let mut total_matches = 0;
    let mut files_analyzed = 0;

    // Walk through files
    let walker = WalkDir::new(&request.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in walker {
        let path = entry.path();

        // Skip non-source files
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !is_supported_language_file(ext) {
            continue;
        }

        // Get analyzer for the file
        let analyzer = match get_analyzer_for_file(path)? {
            Some(analyzer) => analyzer,
            None => continue, // No analyzer for this file type
        };

        // Read file content
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        files_analyzed += 1;

        // Create semantic context
        let context = SemanticContext::new(
            path.to_path_buf(),
            request.path.clone(),
            3, // max depth
        );

        // Analyze file
        if let Ok(analysis) = analyzer.analyze_file(path, &content, &context) {
            match request.search_type {
                super::SemanticSearchType::Functions => {
                    // Search for function definitions
                    for func in &analysis.exported_functions {
                        if func
                            .name
                            .to_lowercase()
                            .contains(&request.query.to_lowercase())
                        {
                            total_matches += 1;

                            let result = super::SemanticSearchResult {
                                file_path: path.to_path_buf(),
                                symbol_name: func.name.clone(),
                                symbol_type: "function".to_string(),
                                line_number: func.line + 1, // Convert to 1-based
                                context: format!(
                                    "{}function {}",
                                    if func.is_exported { "pub " } else { "" },
                                    func.name
                                ),
                            };

                            results.push(result);

                            if let Some(max) = request.max_results {
                                if results.len() >= max as usize {
                                    break;
                                }
                            }
                        }
                    }
                }
                super::SemanticSearchType::Types => {
                    // Search for type references
                    // Note: This searches for type usage, not definitions
                    // Full type definition search would require AST parsing
                    for typ in &analysis.type_references {
                        if typ
                            .name
                            .to_lowercase()
                            .contains(&request.query.to_lowercase())
                        {
                            total_matches += 1;

                            let result = super::SemanticSearchResult {
                                file_path: path.to_path_buf(),
                                symbol_name: typ.name.clone(),
                                symbol_type: "type".to_string(),
                                line_number: typ.line + 1,
                                context: format!("Type reference: {}", typ.name),
                            };

                            results.push(result);

                            if let Some(max) = request.max_results {
                                if results.len() >= max as usize {
                                    break;
                                }
                            }
                        }
                    }
                }
                super::SemanticSearchType::Imports => {
                    // Search for imports
                    for import in &analysis.imports {
                        if import
                            .module
                            .to_lowercase()
                            .contains(&request.query.to_lowercase())
                            || import.items.iter().any(|item| {
                                item.to_lowercase().contains(&request.query.to_lowercase())
                            })
                        {
                            total_matches += 1;

                            let context = if import.items.is_empty() {
                                format!("import {}", import.module)
                            } else {
                                format!(
                                    "import {{ {} }} from {}",
                                    import.items.join(", "),
                                    import.module
                                )
                            };

                            let result = super::SemanticSearchResult {
                                file_path: path.to_path_buf(),
                                symbol_name: import.module.clone(),
                                symbol_type: "import".to_string(),
                                line_number: import.line + 1,
                                context,
                            };

                            results.push(result);

                            if let Some(max) = request.max_results {
                                if results.len() >= max as usize {
                                    break;
                                }
                            }
                        }
                    }
                }
                super::SemanticSearchType::References => {
                    // Search for function calls that match the query
                    for call in &analysis.function_calls {
                        if call
                            .name
                            .to_lowercase()
                            .contains(&request.query.to_lowercase())
                        {
                            total_matches += 1;

                            let result = super::SemanticSearchResult {
                                file_path: path.to_path_buf(),
                                symbol_name: call.name.clone(),
                                symbol_type: "reference".to_string(),
                                line_number: call.line + 1,
                                context: format!("Function call: {}", call.name),
                            };

                            results.push(result);

                            if let Some(max) = request.max_results {
                                if results.len() >= max as usize {
                                    break;
                                }
                            }
                        }
                    }

                    // Also search type references
                    for typ in &analysis.type_references {
                        if typ
                            .name
                            .to_lowercase()
                            .contains(&request.query.to_lowercase())
                        {
                            total_matches += 1;

                            let result = super::SemanticSearchResult {
                                file_path: path.to_path_buf(),
                                symbol_name: typ.name.clone(),
                                symbol_type: "reference".to_string(),
                                line_number: typ.line + 1,
                                context: format!("Type usage: {}", typ.name),
                            };

                            results.push(result);

                            if let Some(max) = request.max_results {
                                if results.len() >= max as usize {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Stop if we've hit the max results
        if let Some(max) = request.max_results {
            if results.len() >= max as usize {
                break;
            }
        }
    }

    let search_time_ms = start.elapsed().as_millis() as u64;

    Ok(super::SemanticSearchResponse {
        results,
        total_matches,
        files_analyzed,
        search_time_ms,
    })
}

/// Check if file extension is a supported language
fn is_supported_language_file(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "cc"
            | "cxx"
            | "h"
            | "hpp"
            | "cs"
            | "rb"
            | "php"
            | "swift"
            | "kt"
            | "kts"
            | "scala"
            | "r"
            | "lua"
            | "dart"
            | "jl"
            | "hs"
            | "elm"
            | "clj"
            | "cljs"
            | "ex"
            | "exs"
            | "ml"
            | "mli"
            | "nim"
            | "zig"
    )
}

/// Execute LLM with prompt and context
fn execute_llm_sync(prompt: &str, context: &str, llm_tool: Option<&str>) -> Result<String> {
    use crate::cli::LlmTool;
    use crate::utils::error::ContextCreatorError;
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Determine which LLM tool to use
    let tool = if let Some(tool_str) = llm_tool {
        match tool_str {
            "gemini" => LlmTool::Gemini,
            "codex" => LlmTool::Codex,
            _ => LlmTool::Gemini, // Default to gemini for unknown tools
        }
    } else {
        LlmTool::Gemini // Default
    };

    let full_input = format!("{prompt}\n\n{context}");
    let tool_command = tool.command();

    let mut child = Command::new(tool_command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ContextCreatorError::LlmToolNotFound {
                    tool: tool_command.to_string(),
                    install_instructions: tool.install_instructions().to_string(),
                }
            } else {
                ContextCreatorError::SubprocessError(e.to_string())
            }
        })?;

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(full_input.as_bytes())?;
        stdin.flush()?;
    }

    // Capture output
    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ContextCreatorError::SubprocessError(format!(
            "{tool_command} failed: {stderr}"
        ))
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
