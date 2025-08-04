// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use opentelemetry::{trace::{Tracer, Span, Status}, global, KeyValue};
use std::collections::HashMap;
use std::time::Duration;

/// Service for analyzing code repositories
pub struct AnalyzerService {
    tracer: Box<dyn Tracer>,
    cache: HashMap<String, AnalysisResult>,
}

#[derive(Clone, Debug)]
pub struct AnalysisResult {
    pub total_files: usize,
    pub total_lines: usize,
    pub languages: Vec<String>,
}

impl AnalyzerService {
    pub fn new() -> Self {
        Self {
            tracer: global::tracer("analyzer-service"),
            cache: HashMap::new(),
        }
    }

    /// Analyze a repository and return analysis results
    pub fn analyze_repository(&mut self, repo_path: &str) -> Result<AnalysisResult, String> {
        let mut span = self.tracer.start("analyze_repository");
        span.set_attribute(KeyValue::new("repo.path", repo_path.to_string()));

        // Check cache first
        if let Some(cached) = self.check_cache(repo_path) {
            span.set_attribute(KeyValue::new("cache.hit", true));
            return Ok(cached);
        }

        // Scan files
        let files = match self.scan_directory(repo_path) {
            Ok(files) => files,
            Err(e) => {
                span.record_error(&e);
                span.set_status(Status::error(format!("Failed to scan directory: {}", e)));
                return Err(format!("Scan failed: {}", e));
            }
        };

        span.set_attribute(KeyValue::new("files.count", files.len() as i64));

        // Analyze each file
        let mut total_lines = 0;
        let mut languages = Vec::new();
        
        for file in &files {
            let file_result = self.analyze_file(file)?;
            total_lines += file_result.lines;
            if !languages.contains(&file_result.language) {
                languages.push(file_result.language);
            }
        }

        let result = AnalysisResult {
            total_files: files.len(),
            total_lines,
            languages,
        };

        // Cache the result
        self.cache.insert(repo_path.to_string(), result.clone());
        
        span.set_attribute(KeyValue::new("analysis.total_lines", total_lines as i64));
        span.set_attribute(KeyValue::new("analysis.languages", languages.len() as i64));

        Ok(result)
    }

    /// Check if we have cached results
    fn check_cache(&self, repo_path: &str) -> Option<AnalysisResult> {
        let span = self.tracer.start("check_cache");
        
        // Simulate cache lookup
        std::thread::sleep(Duration::from_millis(2));
        
        self.cache.get(repo_path).cloned()
    }

    /// Scan directory for files
    fn scan_directory(&self, path: &str) -> Result<Vec<String>, std::io::Error> {
        let mut span = self.tracer.start("scan_directory");
        span.set_attribute(KeyValue::new("path", path.to_string()));
        
        // Simulate directory scanning
        std::thread::sleep(Duration::from_millis(15));
        
        // Return mock files for testing
        Ok(vec![
            format!("{}/main.rs", path),
            format!("{}/lib.rs", path),
            format!("{}/config.toml", path),
        ])
    }

    /// Analyze a single file
    fn analyze_file(&self, file_path: &str) -> Result<FileAnalysis, String> {
        let mut span = self.tracer.start("analyze_file");
        span.set_attribute(KeyValue::new("file.path", file_path.to_string()));
        
        // Simulate file analysis
        std::thread::sleep(Duration::from_millis(5));
        
        let language = if file_path.ends_with(".rs") {
            "rust"
        } else if file_path.ends_with(".toml") {
            "toml"
        } else {
            "unknown"
        };
        
        let lines = 100; // Mock line count
        
        span.set_attribute(KeyValue::new("file.language", language));
        span.set_attribute(KeyValue::new("file.lines", lines));
        
        Ok(FileAnalysis {
            language: language.to_string(),
            lines,
        })
    }

    /// Generate summary report
    pub fn generate_report(&self, results: &[AnalysisResult]) -> String {
        let span = self.tracer.start("generate_report");
        
        let total_repos = results.len();
        let total_files: usize = results.iter().map(|r| r.total_files).sum();
        let total_lines: usize = results.iter().map(|r| r.total_lines).sum();
        
        // Simulate report generation
        std::thread::sleep(Duration::from_millis(10));
        
        format!(
            "Analysis Report:\n- Repositories: {}\n- Total Files: {}\n- Total Lines: {}",
            total_repos, total_files, total_lines
        )
    }
}

struct FileAnalysis {
    language: String,
    lines: usize,
}