//! Generate enriched source code output with telemetry data

use std::collections::HashMap;
use std::path::Path;

use crate::core::telemetry::correlator::{CorrelationKey, CorrelationResult};
use crate::core::telemetry::types::FunctionMetrics;
use crate::utils::error::ContextCreatorError;

/// Enriches source code with telemetry data
pub struct TelemetryEnricher {
    /// Correlation results containing function metrics
    correlation_result: CorrelationResult,
}

impl TelemetryEnricher {
    /// Create a new enricher with correlation results
    pub fn new(correlation_result: CorrelationResult) -> Self {
        Self { correlation_result }
    }

    /// Generate enrichment comment for a function
    pub fn generate_function_comment(&self, key: &CorrelationKey) -> Option<String> {
        let metrics = self.correlation_result.function_metrics.get(key)?;

        // Calculate percentiles
        let p50 = metrics.percentile_immutable(50.0);
        let p95 = metrics.percentile_immutable(95.0);
        let p99 = metrics.percentile_immutable(99.0);

        let mut comment = String::from("<!-- OpenTelemetry Metrics\n");

        // Add call count
        comment.push_str(&format!("Calls: {} (last 24h)\n", metrics.call_count));

        // Add latency metrics if available
        if !metrics.latencies.is_empty() {
            comment.push_str("Latency: ");
            if let Some(p50) = p50 {
                comment.push_str(&format!("p50={p50:.0}ms"));
            }
            if let Some(p95) = p95 {
                comment.push_str(&format!(", p95={p95:.0}ms"));
            }
            if let Some(p99) = p99 {
                comment.push_str(&format!(", p99={p99:.0}ms"));
            }
            comment.push('\n');
        }

        // Add error rate if errors occurred
        if metrics.error_count > 0 {
            let error_rate = (metrics.error_count as f64 / metrics.call_count as f64) * 100.0;
            comment.push_str(&format!("Error Rate: {error_rate:.1}%\n"));

            if let Some(error_msg) = &metrics.common_error {
                comment.push_str(&format!("Most Common Error: \"{error_msg}\"\n"));
            }
        }

        comment.push_str("-->");

        Some(comment)
    }

    /// Enrich a source file with telemetry data
    pub fn enrich_file(
        &self,
        file_path: &Path,
        content: &str,
    ) -> Result<String, ContextCreatorError> {
        // For MVP, we'll add comments at the beginning of the file
        // In a full implementation, we'd parse the file and insert comments at function locations

        let mut enriched = String::new();

        // Find all metrics for this file
        let file_metrics: Vec<(&CorrelationKey, &FunctionMetrics)> = self
            .correlation_result
            .function_metrics
            .iter()
            .filter(|(key, _)| key.file_path == file_path)
            .collect();

        if !file_metrics.is_empty() {
            enriched.push_str("<!-- OpenTelemetry Metrics Summary\n");
            enriched.push_str(&format!("File: {}\n", file_path.display()));
            enriched.push_str(&format!("Functions with metrics: {}\n", file_metrics.len()));
            enriched.push_str("-->\n\n");
        }

        // Add the original content
        enriched.push_str(content);

        Ok(enriched)
    }

    /// Generate a summary of all telemetry data
    pub fn generate_summary(&self) -> String {
        let mut summary = String::from("# OpenTelemetry Metrics Summary\n\n");

        summary.push_str(&format!(
            "Total functions with metrics: {}\n",
            self.correlation_result.function_metrics.len()
        ));
        summary.push_str(&format!(
            "Uncorrelated spans: {}\n\n",
            self.correlation_result.uncorrelated_spans.len()
        ));

        // Group metrics by file
        let mut by_file: HashMap<&Path, Vec<(&CorrelationKey, &FunctionMetrics)>> = HashMap::new();
        for (key, metrics) in &self.correlation_result.function_metrics {
            by_file
                .entry(&key.file_path)
                .or_default()
                .push((key, metrics));
        }

        // Generate per-file summaries
        for (file_path, functions) in by_file {
            summary.push_str(&format!("## {}\n", file_path.display()));

            for (key, metrics) in functions {
                summary.push_str(&format!("\n### Function: {}\n", key.function_name));

                if let Some(line) = key.line_number {
                    summary.push_str(&format!("Line: {line}\n"));
                }

                summary.push_str(&format!("- Calls: {}\n", metrics.call_count));

                if !metrics.latencies.is_empty() {
                    if let Some(p50) = metrics.percentile_immutable(50.0) {
                        summary.push_str(&format!("- Latency p50: {p50:.0}ms\n"));
                    }
                    if let Some(p95) = metrics.percentile_immutable(95.0) {
                        summary.push_str(&format!("- Latency p95: {p95:.0}ms\n"));
                    }
                }

                if metrics.error_count > 0 {
                    let error_rate =
                        (metrics.error_count as f64 / metrics.call_count as f64) * 100.0;
                    summary.push_str(&format!("- Error rate: {error_rate:.1}%\n"));
                }
            }
            summary.push('\n');
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_correlation_result() -> CorrelationResult {
        let mut function_metrics = HashMap::new();

        // Add test metrics
        let key = CorrelationKey {
            file_path: PathBuf::from("/project/src/api/handlers.rs"),
            function_name: "process_payment".to_string(),
            line_number: Some(42),
        };

        let mut metrics = FunctionMetrics::new();
        metrics.call_count = 1234;
        // Use add_latency to populate data properly
        for latency in [45.0, 50.0, 120.0, 130.0, 250.0] {
            metrics.add_latency(latency);
        }
        metrics.error_count = 4;
        metrics.common_error = Some("Payment gateway timeout".to_string());

        function_metrics.insert(key, metrics);

        CorrelationResult {
            function_metrics,
            correlated_count: 1,
            uncorrelated_spans: vec![],
        }
    }

    #[test]
    fn test_generate_function_comment() {
        let result = create_test_correlation_result();
        let enricher = TelemetryEnricher::new(result);

        let key = CorrelationKey {
            file_path: PathBuf::from("/project/src/api/handlers.rs"),
            function_name: "process_payment".to_string(),
            line_number: Some(42),
        };

        let comment = enricher.generate_function_comment(&key).unwrap();

        println!("Generated comment:\n{comment}");

        assert!(comment.contains("<!-- OpenTelemetry Metrics"));
        assert!(comment.contains("Calls: 1234"));
        assert!(comment.contains("p50=120ms")); // Fixed: p50 of [45, 50, 120, 130, 250] is 120
        assert!(comment.contains("p95=250ms")); // Fixed: p95 of 5 values is the 5th value
        assert!(comment.contains("Error Rate: 0.3%"));
        assert!(comment.contains("Payment gateway timeout"));
        assert!(comment.contains("-->"));
    }

    #[test]
    fn test_generate_function_comment_no_errors() {
        let mut result = create_test_correlation_result();

        // Modify to have no errors
        if let Some(metrics) = result.function_metrics.values_mut().next() {
            metrics.error_count = 0;
            metrics.common_error = None;
        }

        let enricher = TelemetryEnricher::new(result);

        let key = CorrelationKey {
            file_path: PathBuf::from("/project/src/api/handlers.rs"),
            function_name: "process_payment".to_string(),
            line_number: Some(42),
        };

        let comment = enricher.generate_function_comment(&key).unwrap();

        assert!(!comment.contains("Error Rate"));
        assert!(!comment.contains("Most Common Error"));
    }

    #[test]
    fn test_enrich_file() {
        let result = create_test_correlation_result();
        let enricher = TelemetryEnricher::new(result);

        let file_path = PathBuf::from("/project/src/api/handlers.rs");
        let content = "async fn process_payment() -> Result<()> {\n    Ok(())\n}";

        let enriched = enricher.enrich_file(&file_path, content).unwrap();

        assert!(enriched.contains("<!-- OpenTelemetry Metrics Summary"));
        assert!(enriched.contains("Functions with metrics: 1"));
        assert!(enriched.contains(content));
    }

    #[test]
    fn test_generate_summary() {
        let result = create_test_correlation_result();
        let enricher = TelemetryEnricher::new(result);

        let summary = enricher.generate_summary();

        assert!(summary.contains("# OpenTelemetry Metrics Summary"));
        assert!(summary.contains("Total functions with metrics: 1"));
        assert!(summary.contains("## /project/src/api/handlers.rs"));
        assert!(summary.contains("### Function: process_payment"));
        assert!(summary.contains("Line: 42"));
        assert!(summary.contains("Calls: 1234"));
        assert!(summary.contains("Latency p50: 120ms")); // Fixed: p50 of [45, 50, 120, 130, 250] is 120
        assert!(summary.contains("Error rate: 0.3%"));
    }
}
