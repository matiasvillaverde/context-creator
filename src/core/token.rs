//! Token counting functionality using tiktoken-rs

use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiktoken_rs::{cl100k_base, CoreBPE};

/// Token counter with caching support
pub struct TokenCounter {
    /// The tiktoken encoder
    encoder: Arc<CoreBPE>,
    /// Cache of token counts for content hashes
    cache: Arc<Mutex<HashMap<u64, usize>>>,
}

impl TokenCounter {
    /// Create a new token counter with cl100k_base encoding (GPT-4)
    pub fn new() -> Result<Self> {
        let encoder = cl100k_base()?;
        Ok(TokenCounter { encoder: Arc::new(encoder), cache: Arc::new(Mutex::new(HashMap::new())) })
    }

    /// Count tokens in a single text
    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        // Calculate hash for caching
        let hash = calculate_hash(text);

        // Check cache first
        if let Ok(cache) = self.cache.lock() {
            if let Some(&count) = cache.get(&hash) {
                return Ok(count);
            }
        }

        // Count tokens
        let tokens = self.encoder.encode_with_special_tokens(text);
        let count = tokens.len();

        // Store in cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(hash, count);
        }

        Ok(count)
    }

    /// Count tokens in multiple texts in parallel
    pub fn count_tokens_parallel(&self, texts: &[String]) -> Result<Vec<usize>> {
        texts.par_iter().map(|text| self.count_tokens(text)).collect()
    }

    /// Count tokens for a file's content with metadata
    pub fn count_file_tokens(&self, content: &str, path: &str) -> Result<FileTokenCount> {
        let content_tokens = self.count_tokens(content)?;

        // Count tokens in the file path/header that will be included in markdown
        let header = format!("## {}\n\n```\n", path);
        let footer = "\n```\n\n";
        let header_tokens = self.count_tokens(&header)?;
        let footer_tokens = self.count_tokens(footer)?;

        Ok(FileTokenCount {
            content_tokens,
            overhead_tokens: header_tokens + footer_tokens,
            total_tokens: content_tokens + header_tokens + footer_tokens,
        })
    }

    /// Estimate tokens for multiple files
    pub fn estimate_total_tokens(&self, files: &[(String, String)]) -> Result<TotalTokenEstimate> {
        let mut total_content = 0;
        let mut total_overhead = 0;
        let mut file_counts = Vec::new();

        for (path, content) in files {
            let count = self.count_file_tokens(content, path)?;
            total_content += count.content_tokens;
            total_overhead += count.overhead_tokens;
            file_counts.push((path.clone(), count));
        }

        Ok(TotalTokenEstimate {
            total_tokens: total_content + total_overhead,
            content_tokens: total_content,
            overhead_tokens: total_overhead,
            file_counts,
        })
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new().expect("Failed to create token counter")
    }
}

/// Token count for a single file
#[derive(Debug, Clone)]
pub struct FileTokenCount {
    /// Tokens in the file content
    pub content_tokens: usize,
    /// Tokens in markdown formatting overhead
    pub overhead_tokens: usize,
    /// Total tokens (content + overhead)
    pub total_tokens: usize,
}

/// Total token estimate for multiple files
#[derive(Debug)]
pub struct TotalTokenEstimate {
    /// Total tokens across all files
    pub total_tokens: usize,
    /// Total content tokens
    pub content_tokens: usize,
    /// Total overhead tokens
    pub overhead_tokens: usize,
    /// Individual file counts
    pub file_counts: Vec<(String, FileTokenCount)>,
}

/// Calculate a hash for content caching
fn calculate_hash(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Check if adding a file would exceed token limit
pub fn would_exceed_limit(current_tokens: usize, file_tokens: usize, max_tokens: usize) -> bool {
    current_tokens + file_tokens > max_tokens
}

/// Calculate remaining token budget
pub fn remaining_tokens(current_tokens: usize, max_tokens: usize) -> usize {
    max_tokens.saturating_sub(current_tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let counter = TokenCounter::new().unwrap();

        // Test simple text
        let count = counter.count_tokens("Hello, world!").unwrap();
        assert!(count > 0);

        // Test empty text
        let count = counter.count_tokens("").unwrap();
        assert_eq!(count, 0);

        // Test caching
        let text = "This is a test text for caching";
        let count1 = counter.count_tokens(text).unwrap();
        let count2 = counter.count_tokens(text).unwrap();
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_file_token_counting() {
        let counter = TokenCounter::new().unwrap();

        let content = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let path = "src/main.rs";

        let count = counter.count_file_tokens(content, path).unwrap();
        assert!(count.content_tokens > 0);
        assert!(count.overhead_tokens > 0);
        assert_eq!(count.total_tokens, count.content_tokens + count.overhead_tokens);
    }

    #[test]
    fn test_parallel_counting() {
        let counter = TokenCounter::new().unwrap();

        let texts =
            vec!["First text".to_string(), "Second text".to_string(), "Third text".to_string()];

        let counts = counter.count_tokens_parallel(&texts).unwrap();
        assert_eq!(counts.len(), 3);
        assert!(counts.iter().all(|&c| c > 0));
    }

    #[test]
    fn test_token_limit_checks() {
        assert!(would_exceed_limit(900, 200, 1000));
        assert!(!would_exceed_limit(800, 200, 1000));

        assert_eq!(remaining_tokens(300, 1000), 700);
        assert_eq!(remaining_tokens(1100, 1000), 0);
    }

    #[test]
    fn test_total_estimation() {
        let counter = TokenCounter::new().unwrap();

        let files = vec![
            ("file1.rs".to_string(), "content1".to_string()),
            ("file2.rs".to_string(), "content2".to_string()),
        ];

        let estimate = counter.estimate_total_tokens(&files).unwrap();
        assert!(estimate.total_tokens > 0);
        assert_eq!(estimate.file_counts.len(), 2);
    }
}
