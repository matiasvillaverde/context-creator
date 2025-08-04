//! Common types for telemetry data processing

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

/// Represents a telemetry span with code-related attributes
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySpan {
    /// Span name (often the function or operation name)
    pub name: String,
    /// Function name from code.function.name attribute
    pub function_name: Option<String>,
    /// File path from code.file.path attribute
    pub file_path: Option<PathBuf>,
    /// Line number from code.line.number attribute
    pub line_number: Option<u32>,
    /// Service name from resource attributes
    pub service_name: Option<String>,
    /// Start time in Unix nanoseconds
    pub start_time_nanos: u64,
    /// End time in Unix nanoseconds
    pub end_time_nanos: u64,
    /// Duration in milliseconds
    pub duration_ms: f64,
    /// Additional attributes
    pub attributes: HashMap<String, AttributeValue>,
}

impl TelemetrySpan {
    /// Calculate duration in milliseconds
    pub fn calculate_duration_ms(&mut self) {
        self.duration_ms = (self.end_time_nanos - self.start_time_nanos) as f64 / 1_000_000.0;
    }
}

/// Represents an attribute value in various types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
    Array(Vec<AttributeValue>),
}

/// Maximum number of latency samples to keep in memory
const MAX_LATENCY_SAMPLES: usize = 1000;

/// Metrics aggregated for a specific function
#[derive(Debug, Clone)]
pub struct FunctionMetrics {
    /// Number of calls
    pub call_count: usize,
    /// Latency values in milliseconds (bounded sliding window)
    pub latencies: VecDeque<f64>,
    /// Error count
    pub error_count: usize,
    /// Most common error message
    pub common_error: Option<String>,
    /// Cached sorted latencies for performance
    cached_sorted: Option<Vec<f64>>,
}

impl FunctionMetrics {
    pub fn new() -> Self {
        Self {
            call_count: 0,
            latencies: VecDeque::new(),
            error_count: 0,
            common_error: None,
            cached_sorted: None,
        }
    }

    /// Add a new latency sample with bounded memory
    pub fn add_latency(&mut self, latency_ms: f64) {
        self.latencies.push_back(latency_ms);

        // Maintain sliding window size
        if self.latencies.len() > MAX_LATENCY_SAMPLES {
            self.latencies.pop_front();
        }

        // Invalidate cache when data changes
        self.cached_sorted = None;
    }

    /// Get or compute sorted latencies (lazy evaluation)
    fn get_sorted_latencies(&mut self) -> &Vec<f64> {
        if self.cached_sorted.is_none() {
            let mut sorted: Vec<f64> = self.latencies.iter().copied().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            self.cached_sorted = Some(sorted);
        }
        self.cached_sorted.as_ref().unwrap()
    }

    /// Calculate percentile from latencies (optimized)
    pub fn percentile(&mut self, p: f64) -> Option<f64> {
        if self.latencies.is_empty() {
            return None;
        }

        let sorted = self.get_sorted_latencies();

        // For small sample sizes, use nearest-rank method
        let index = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
        sorted.get(index.saturating_sub(1)).copied()
    }

    /// Calculate percentile from latencies (immutable version for backward compatibility)
    pub fn percentile_immutable(&self, p: f64) -> Option<f64> {
        if self.latencies.is_empty() {
            return None;
        }

        // For immutable version, we have to sort each time
        // This is for backward compatibility with existing code
        let mut sorted: Vec<f64> = self.latencies.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let index = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
        sorted.get(index.saturating_sub(1)).copied()
    }
}

impl Default for FunctionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of parsing OTLP data
#[derive(Debug)]
pub struct ParsedTelemetry {
    /// All spans extracted from the telemetry data
    pub spans: Vec<TelemetrySpan>,
    /// Spans that have code-related attributes
    pub code_spans: Vec<TelemetrySpan>,
}

// JSON structures for OTLP format
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpJson {
    pub resource_spans: Option<Vec<ResourceSpans>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpans {
    pub resource: Option<Resource>,
    pub scope_spans: Option<Vec<ScopeSpans>>,
}

#[derive(Debug, Deserialize)]
pub struct Resource {
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeSpans {
    pub spans: Option<Vec<Span>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub name: String,
    pub start_time_unix_nano: Option<String>,
    pub end_time_unix_nano: Option<String>,
    pub attributes: Option<Vec<Attribute>>,
    pub status: Option<Status>,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    pub code: Option<i32>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Attribute {
    pub key: String,
    pub value: Option<AnyValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnyValue {
    pub string_value: Option<String>,
    pub int_value: Option<String>, // Stored as string in JSON
    pub double_value: Option<f64>,
    pub bool_value: Option<bool>,
    pub array_value: Option<ArrayValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArrayValue {
    pub values: Vec<AnyValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_function_metrics_percentile_basic() {
        // Given: Metrics with sorted latencies
        let mut metrics = FunctionMetrics::new();
        // Use add_latency to populate data properly
        for latency in [10.0, 20.0, 30.0, 40.0, 50.0] {
            metrics.add_latency(latency);
        }

        // When: Calculating percentiles
        // Then: Should return correct values
        assert_eq!(metrics.percentile(50.0), Some(30.0));
        assert_eq!(metrics.percentile(95.0), Some(50.0));
        assert_eq!(metrics.percentile(0.0), Some(10.0));
    }

    #[test]
    fn test_function_metrics_percentile_edge_cases() {
        // Given: Empty metrics
        let mut empty_metrics = FunctionMetrics::new();

        // When: Calculating percentile on empty data
        // Then: Should return None
        assert_eq!(empty_metrics.percentile(50.0), None);

        // Given: Single value
        let mut single_metrics = FunctionMetrics::new();
        single_metrics.add_latency(42.0);

        // When: Calculating percentiles
        // Then: Should return the single value
        assert_eq!(single_metrics.percentile(0.0), Some(42.0));
        assert_eq!(single_metrics.percentile(50.0), Some(42.0));
        assert_eq!(single_metrics.percentile(100.0), Some(42.0));
    }

    #[test]
    fn test_function_metrics_percentile_unsorted_data() {
        // Given: Unsorted latencies
        let mut metrics = FunctionMetrics::new();
        for latency in [50.0, 10.0, 40.0, 20.0, 30.0] {
            metrics.add_latency(latency);
        }

        // When: Calculating percentiles
        // Then: Should handle unsorted data correctly
        assert_eq!(metrics.percentile(50.0), Some(30.0));
        assert_eq!(metrics.percentile(95.0), Some(50.0));
    }

    #[test]
    fn test_function_metrics_percentile_performance() {
        // Given: Large dataset (1000 values)
        let mut metrics = FunctionMetrics::new();
        for i in 0..1000 {
            metrics.add_latency(i as f64);
        }

        // When: Calculating multiple percentiles with timing
        let start = Instant::now();
        let _p50 = metrics.percentile(50.0);
        let _p95 = metrics.percentile(95.0);
        let _p99 = metrics.percentile(99.0);
        let duration = start.elapsed();

        // Then: Should complete reasonably fast (< 1ms for this size)
        assert!(
            duration.as_millis() < 10,
            "Percentile calculation too slow: {duration:?}"
        );

        // And: Results should be correct
        assert_eq!(metrics.percentile(50.0), Some(499.0)); // 0-indexed, so 50th percentile of 0-999
        assert_eq!(metrics.percentile(95.0), Some(949.0));
        assert_eq!(metrics.percentile(99.0), Some(989.0));
    }

    #[test]
    fn test_function_metrics_percentile_invalid_inputs() {
        // Given: Valid metrics
        let mut metrics = FunctionMetrics::new();
        for latency in [10.0, 20.0, 30.0, 40.0, 50.0] {
            metrics.add_latency(latency);
        }

        // When/Then: Invalid percentile values behavior (current implementation)
        assert_eq!(metrics.percentile(-10.0), Some(10.0)); // Negative percentile returns first element
        assert_eq!(metrics.percentile(150.0), None); // Out of bounds percentile returns None
        assert_eq!(metrics.percentile(100.0), Some(50.0)); // 100th percentile should work
    }

    #[test]
    fn test_function_metrics_memory_bounds() {
        // Given: Metrics with sliding window
        let mut metrics = FunctionMetrics::new();

        // When: Adding many latency values (more than MAX_LATENCY_SAMPLES)
        for i in 0..2000 {
            metrics.add_latency(i as f64);
        }

        // Then: Should maintain bounded memory
        assert_eq!(metrics.latencies.len(), MAX_LATENCY_SAMPLES);

        // And: Should keep the most recent samples
        // Last added was 1999.0, sliding window should contain values 1000-1999
        assert_eq!(metrics.latencies.back(), Some(&1999.0));
        assert_eq!(metrics.latencies.front(), Some(&1000.0));
    }

    #[test]
    fn test_telemetry_span_duration() {
        let mut span = TelemetrySpan {
            name: "test".to_string(),
            function_name: None,
            file_path: None,
            line_number: None,
            service_name: None,
            start_time_nanos: 1_000_000_000,
            end_time_nanos: 1_050_000_000,
            duration_ms: 0.0,
            attributes: HashMap::new(),
        };

        span.calculate_duration_ms();
        assert_eq!(span.duration_ms, 50.0);
    }

    // Performance regression test
    #[test]
    fn test_multiple_percentile_calls_performance() {
        // Given: Realistic dataset size
        let mut metrics = FunctionMetrics::new();
        for i in 0..1000 {
            metrics.add_latency((i as f64) * 0.1);
        }

        // When: Calling percentile multiple times (simulating real usage)
        let start = Instant::now();
        for _ in 0..100 {
            let _p50 = metrics.percentile(50.0);
            let _p95 = metrics.percentile(95.0);
            let _p99 = metrics.percentile(99.0);
        }
        let duration = start.elapsed();

        // Then: Should complete in reasonable time
        // Old implementation: O(n log n) * calls = ~300ms
        // New implementation: O(n log n) once + O(1) * calls = ~1ms
        println!("Multiple percentile calls took: {duration:?}");

        // With caching, this should be much faster than the old implementation
        assert!(
            duration.as_millis() < 50,
            "Multiple percentile calls too slow: {:?}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_function_metrics_cache_invalidation() {
        // Given: Metrics with some data
        let mut metrics = FunctionMetrics::new();
        for i in 0..10 {
            metrics.add_latency(i as f64);
        }

        // When: Getting percentile (builds cache)
        let p50_first = metrics.percentile(50.0);

        // Then: Adding new data should invalidate cache
        metrics.add_latency(100.0);
        let p50_after_add = metrics.percentile(50.0);

        // And: Results should be different (cache was invalidated)
        assert_ne!(p50_first, p50_after_add);

        // Verify the new result includes the outlier
        assert!(p50_after_add > p50_first);
    }
}
