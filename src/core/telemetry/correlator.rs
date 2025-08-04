//! Correlate telemetry data with source code

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::core::semantic::dependency_types::FileAnalysisResult;
use crate::core::telemetry::types::{FunctionMetrics, TelemetrySpan};

/// Correlates telemetry data with source code
pub struct TelemetryCorrelator {
    /// Analysis results containing function information
    analysis_results: Vec<FileAnalysisResult>,
    /// File paths corresponding to analysis results
    file_paths: Vec<PathBuf>,
    /// Base path for resolving relative paths
    base_path: PathBuf,
    /// Function name index for O(1) exact lookups
    exact_function_index: HashMap<String, CorrelationKey>,
    /// Function name index for O(1) case-insensitive lookups
    case_insensitive_index: HashMap<String, CorrelationKey>,
    /// Suffix index for faster fuzzy matching
    suffix_index: HashMap<String, Vec<CorrelationKey>>,
}

impl TelemetryCorrelator {
    /// Create a new correlator with analysis results
    pub fn new(
        analysis_results: Vec<FileAnalysisResult>,
        file_paths: Vec<PathBuf>,
        base_path: PathBuf,
    ) -> Self {
        let mut exact_function_index = HashMap::new();
        let mut case_insensitive_index = HashMap::new();
        let mut suffix_index: HashMap<String, Vec<CorrelationKey>> = HashMap::new();

        // Build indexes for O(1) lookups
        for analysis in &analysis_results {
            if let Some(file_path) = file_paths.get(analysis.file_index) {
                for func_def in &analysis.exported_functions {
                    let key = CorrelationKey {
                        file_path: file_path.clone(),
                        function_name: func_def.name.clone(),
                        line_number: Some(func_def.line as u32),
                    };

                    // Exact match index
                    exact_function_index.insert(func_def.name.clone(), key.clone());

                    // Case-insensitive index
                    case_insensitive_index.insert(func_def.name.to_lowercase(), key.clone());

                    // Suffix index (for fuzzy matching)
                    // Index each suffix of the function name
                    let name = &func_def.name;
                    for i in 0..name.len() {
                        let suffix = name[i..].to_string();
                        suffix_index.entry(suffix).or_default().push(key.clone());
                    }
                }
            }
        }

        Self {
            analysis_results,
            file_paths,
            base_path,
            exact_function_index,
            case_insensitive_index,
            suffix_index,
        }
    }

    /// Correlate telemetry spans with source code
    pub fn correlate_spans(&self, spans: Vec<TelemetrySpan>) -> CorrelationResult {
        let mut function_metrics: HashMap<CorrelationKey, FunctionMetrics> = HashMap::new();
        let mut correlated_count = 0;
        let mut uncorrelated_spans = Vec::new();

        for span in spans {
            if let Some(key) = self.correlate_span(&span) {
                correlated_count += 1;

                // Update metrics for this function
                let metrics = function_metrics.entry(key).or_default();
                metrics.call_count += 1;
                metrics.add_latency(span.duration_ms);

                // Check if this span represents an error
                if let Some(crate::core::telemetry::types::AttributeValue::Int(code)) =
                    span.attributes.get("status.code")
                {
                    if *code == 2 {
                        // OTEL STATUS_CODE_ERROR
                        metrics.error_count += 1;
                        // Extract error message if available
                        if let Some(crate::core::telemetry::types::AttributeValue::String(
                            error_msg,
                        )) = span.attributes.get("status.message")
                        {
                            metrics.common_error = Some(error_msg.clone());
                        }
                    }
                }
            } else {
                uncorrelated_spans.push(span);
            }
        }

        CorrelationResult {
            function_metrics,
            correlated_count,
            uncorrelated_spans,
        }
    }

    /// Try to correlate a single span with source code
    fn correlate_span(&self, span: &TelemetrySpan) -> Option<CorrelationKey> {
        // Try direct correlation first
        if let Some(ref function_name) = span.function_name {
            if let Some(ref file_path) = span.file_path {
                let resolved_path = self.resolve_path(file_path);

                // Check if this file and function exist in our project
                if self.verify_correlation(&resolved_path, function_name) {
                    return Some(CorrelationKey {
                        file_path: resolved_path,
                        function_name: function_name.clone(),
                        line_number: span.line_number,
                    });
                }
            }
        }

        // Try fuzzy matching if direct correlation fails
        if let Some(ref function_name) = span.function_name {
            if let Some(key) = self.find_function_fuzzy(function_name) {
                debug!("Fuzzy matched function '{}' to {:?}", function_name, key);
                return Some(key);
            }
        }

        // Try to match by span name if no function name
        if span.function_name.is_none() {
            if let Some(key) = self.find_function_fuzzy(&span.name) {
                debug!("Matched span name '{}' to function {:?}", span.name, key);
                return Some(key);
            }
        }

        None
    }

    /// Resolve a path relative to the base path
    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_path.join(path)
        }
    }

    /// Verify that a file and function exist in the project
    fn verify_correlation(&self, file_path: &Path, function_name: &str) -> bool {
        // Find the file in our list
        if let Some(file_idx) = self.file_paths.iter().position(|p| p == file_path) {
            // Check if we have analysis for this file
            if let Some(analysis) = self
                .analysis_results
                .iter()
                .find(|a| a.file_index == file_idx)
            {
                // Check if function exists in exported functions
                return analysis
                    .exported_functions
                    .iter()
                    .any(|def| def.name == function_name);
            }
        }
        false
    }

    /// Find a function using fuzzy matching (optimized with indexes)
    fn find_function_fuzzy(&self, name: &str) -> Option<CorrelationKey> {
        // Try exact match first (O(1))
        if let Some(key) = self.exact_function_index.get(name) {
            return Some(key.clone());
        }

        // Try case-insensitive match (O(1))
        let name_lower = name.to_lowercase();
        if let Some(key) = self.case_insensitive_index.get(&name_lower) {
            return Some(key.clone());
        }

        // Try suffix matching using index (O(1) lookup + small list iteration)
        if let Some(keys) = self.suffix_index.get(name) {
            // Return first match - could be improved with better ranking
            return keys.first().cloned();
        }

        // Try reverse suffix matching (name is suffix of function name)
        // For "PaymentService::process_payment" matching "process_payment"
        for (func_name, key) in &self.exact_function_index {
            if name.ends_with(func_name) || func_name.ends_with(name) {
                return Some(key.clone());
            }
        }

        None
    }
}

/// Key for correlating telemetry data with source code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CorrelationKey {
    pub file_path: PathBuf,
    pub function_name: String,
    pub line_number: Option<u32>,
}

/// Result of correlation process
#[derive(Debug)]
pub struct CorrelationResult {
    /// Metrics aggregated by function
    pub function_metrics: HashMap<CorrelationKey, FunctionMetrics>,
    /// Number of successfully correlated spans
    pub correlated_count: usize,
    /// Spans that couldn't be correlated
    pub uncorrelated_spans: Vec<TelemetrySpan>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::semantic::analyzer::FunctionDefinition;

    fn create_test_analysis() -> (Vec<FileAnalysisResult>, Vec<PathBuf>) {
        let result = FileAnalysisResult {
            file_index: 0,
            imports: vec![],
            function_calls: vec![],
            type_references: vec![],
            exported_functions: vec![FunctionDefinition {
                name: "process_payment".to_string(),
                is_exported: true,
                line: 42,
            }],
            content_hash: None,
            error: None,
        };

        let file_paths = vec![PathBuf::from("/project/src/api/handlers.rs")];

        (vec![result], file_paths)
    }

    #[test]
    fn test_correlate_exact_match() {
        let (analysis_results, file_paths) = create_test_analysis();
        let correlator =
            TelemetryCorrelator::new(analysis_results, file_paths, PathBuf::from("/project"));

        let span = TelemetrySpan {
            name: "process_payment".to_string(),
            function_name: Some("process_payment".to_string()),
            file_path: Some(PathBuf::from("src/api/handlers.rs")),
            line_number: Some(42),
            service_name: Some("payment-api".to_string()),
            start_time_nanos: 1000,
            end_time_nanos: 2000,
            duration_ms: 1.0,
            attributes: HashMap::new(),
        };

        let result = correlator.correlate_spans(vec![span]);

        assert_eq!(result.correlated_count, 1);
        assert_eq!(result.uncorrelated_spans.len(), 0);
        assert_eq!(result.function_metrics.len(), 1);

        let key = result.function_metrics.keys().next().unwrap();
        assert_eq!(key.function_name, "process_payment");
        assert_eq!(key.file_path, PathBuf::from("/project/src/api/handlers.rs"));
    }

    #[test]
    fn test_correlate_fuzzy_match() {
        let (analysis_results, file_paths) = create_test_analysis();
        let correlator =
            TelemetryCorrelator::new(analysis_results, file_paths, PathBuf::from("/project"));

        let span = TelemetrySpan {
            name: "PaymentService::process_payment".to_string(),
            function_name: Some("PaymentService::process_payment".to_string()),
            file_path: None, // Missing file path
            line_number: None,
            service_name: Some("payment-api".to_string()),
            start_time_nanos: 1000,
            end_time_nanos: 2000,
            duration_ms: 1.0,
            attributes: HashMap::new(),
        };

        let result = correlator.correlate_spans(vec![span]);

        assert_eq!(result.correlated_count, 1);
        assert_eq!(result.function_metrics.len(), 1);

        let key = result.function_metrics.keys().next().unwrap();
        assert_eq!(key.function_name, "process_payment");
    }

    #[test]
    fn test_correlate_by_span_name() {
        let (analysis_results, file_paths) = create_test_analysis();
        let correlator =
            TelemetryCorrelator::new(analysis_results, file_paths, PathBuf::from("/project"));

        let span = TelemetrySpan {
            name: "process_payment".to_string(),
            function_name: None, // No function name attribute
            file_path: None,
            line_number: None,
            service_name: Some("payment-api".to_string()),
            start_time_nanos: 1000,
            end_time_nanos: 2000,
            duration_ms: 1.0,
            attributes: HashMap::new(),
        };

        let result = correlator.correlate_spans(vec![span]);

        assert_eq!(result.correlated_count, 1);
        assert_eq!(result.function_metrics.len(), 1);
    }

    #[test]
    fn test_uncorrelated_spans() {
        let (analysis_results, file_paths) = create_test_analysis();
        let correlator =
            TelemetryCorrelator::new(analysis_results, file_paths, PathBuf::from("/project"));

        let span = TelemetrySpan {
            name: "database_query".to_string(),
            function_name: Some("unknown_function".to_string()),
            file_path: None,
            line_number: None,
            service_name: Some("payment-api".to_string()),
            start_time_nanos: 1000,
            end_time_nanos: 2000,
            duration_ms: 1.0,
            attributes: HashMap::new(),
        };

        let result = correlator.correlate_spans(vec![span]);

        assert_eq!(result.correlated_count, 0);
        assert_eq!(result.uncorrelated_spans.len(), 1);
        assert_eq!(result.function_metrics.len(), 0);
    }

    #[test]
    fn test_metrics_aggregation() {
        let (analysis_results, file_paths) = create_test_analysis();
        let correlator =
            TelemetryCorrelator::new(analysis_results, file_paths, PathBuf::from("/project"));

        let spans = vec![
            TelemetrySpan {
                name: "process_payment".to_string(),
                function_name: Some("process_payment".to_string()),
                file_path: Some(PathBuf::from("src/api/handlers.rs")),
                line_number: Some(42),
                service_name: Some("payment-api".to_string()),
                start_time_nanos: 1000,
                end_time_nanos: 2000,
                duration_ms: 1.0,
                attributes: HashMap::new(),
            },
            TelemetrySpan {
                name: "process_payment".to_string(),
                function_name: Some("process_payment".to_string()),
                file_path: Some(PathBuf::from("src/api/handlers.rs")),
                line_number: Some(42),
                service_name: Some("payment-api".to_string()),
                start_time_nanos: 3000,
                end_time_nanos: 5000,
                duration_ms: 2.0,
                attributes: HashMap::new(),
            },
        ];

        let result = correlator.correlate_spans(spans);

        assert_eq!(result.correlated_count, 2);

        let metrics = result.function_metrics.values().next().unwrap();
        assert_eq!(metrics.call_count, 2);
        assert_eq!(metrics.latencies.len(), 2);
        assert_eq!(metrics.latencies.front(), Some(&1.0));
        assert_eq!(metrics.latencies.get(1), Some(&2.0));
    }

    fn create_large_test_correlator(
        files: usize,
        functions_per_file: usize,
    ) -> TelemetryCorrelator {
        let mut analysis_results = Vec::new();
        let mut file_paths = Vec::new();

        for file_idx in 0..files {
            let mut exported_functions = Vec::new();

            for func_idx in 0..functions_per_file {
                exported_functions.push(FunctionDefinition {
                    name: format!("function_{file_idx}_{func_idx}"),
                    is_exported: true,
                    line: func_idx * 10 + 5,
                });
            }

            analysis_results.push(FileAnalysisResult {
                file_index: file_idx,
                imports: vec![],
                function_calls: vec![],
                type_references: vec![],
                exported_functions,
                content_hash: None,
                error: None,
            });

            file_paths.push(PathBuf::from(format!("/project/src/file_{file_idx}.rs")));
        }

        TelemetryCorrelator::new(analysis_results, file_paths, PathBuf::from("/project"))
    }

    #[test]
    fn test_correlation_performance_o_n_squared_problem() {
        use std::time::Instant;

        // Create a large dataset to show O(n²) performance issue
        let correlator = create_large_test_correlator(100, 50); // 100 files × 50 functions = 5000 functions

        // Measure time for multiple lookups (simulating real correlation)
        let start = Instant::now();
        for i in 0..100 {
            let function_name = format!("function_{}_{}", i % 100, i % 50);
            let _key = correlator.find_function_fuzzy(&function_name);
        }
        let duration = start.elapsed();

        println!("O(n²) correlation took: {duration:?} for 100 lookups on 5000 functions");

        // This test shows the performance problem - should be much faster with indexing
        // Current implementation: O(n²) per lookup = O(n² × lookups)
        // Fixed implementation should be: O(n) build index + O(1) per lookup
        assert!(
            duration.as_millis() < 50,
            "Correlation too slow: {duration:?}ms - O(n²) algorithm detected"
        );
    }

    #[test]
    fn test_exact_vs_fuzzy_matching_priority() {
        // Create correlator with functions that could match both exact and fuzzy
        let analysis_result = FileAnalysisResult {
            file_index: 0,
            imports: vec![],
            function_calls: vec![],
            type_references: vec![],
            exported_functions: vec![
                FunctionDefinition {
                    name: "payment".to_string(),
                    is_exported: true,
                    line: 10,
                },
                FunctionDefinition {
                    name: "process_payment".to_string(),
                    is_exported: true,
                    line: 20,
                },
            ],
            content_hash: None,
            error: None,
        };

        let file_paths = vec![PathBuf::from("/project/src/test.rs")];
        let correlator =
            TelemetryCorrelator::new(vec![analysis_result], file_paths, PathBuf::from("/project"));

        // Should prefer exact match over fuzzy match
        let key = correlator.find_function_fuzzy("payment");
        assert!(key.is_some());
        let key = key.unwrap();
        assert_eq!(key.function_name, "payment"); // Should match exact "payment", not fuzzy "process_payment"
        assert_eq!(key.line_number, Some(10));
    }
}
