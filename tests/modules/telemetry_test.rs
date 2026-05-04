//! Integration tests for telemetry command functionality

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn valid_protobuf_trace_bytes() -> Vec<u8> {
    use context_creator::core::telemetry::otlp_parser::{
        any_value, AnyValue, ExportTraceServiceRequest, KeyValue, ProtobufSpan, Resource,
        ResourceSpans, ScopeSpans,
    };
    use prost::Message;

    let otlp = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(Resource {
                attributes: vec![KeyValue {
                    key: "service.name".to_string(),
                    value: Some(AnyValue {
                        value: Some(any_value::Value::StringValue("payment-api".to_string())),
                    }),
                }],
            }),
            scope_spans: vec![ScopeSpans {
                spans: vec![ProtobufSpan {
                    name: "process_payment".to_string(),
                    start_time_unix_nano: 1_704_067_200_000_000_000,
                    end_time_unix_nano: 1_704_067_200_050_000_000,
                    attributes: vec![
                        KeyValue {
                            key: "code.function.name".to_string(),
                            value: Some(AnyValue {
                                value: Some(any_value::Value::StringValue(
                                    "process_payment".to_string(),
                                )),
                            }),
                        },
                        KeyValue {
                            key: "code.file.path".to_string(),
                            value: Some(AnyValue {
                                value: Some(any_value::Value::StringValue(
                                    "src/api/handlers.rs".to_string(),
                                )),
                            }),
                        },
                        KeyValue {
                            key: "code.line.number".to_string(),
                            value: Some(AnyValue {
                                value: Some(any_value::Value::IntValue(1)),
                            }),
                        },
                    ],
                    status: None,
                }],
            }],
        }],
    };

    let mut protobuf_data = Vec::new();
    otlp.encode(&mut protobuf_data).unwrap();
    protobuf_data
}

fn write_valid_protobuf_trace(path: &Path) {
    fs::write(path, valid_protobuf_trace_bytes()).unwrap();
}

#[test]
fn test_parse_otlp_json_format() {
    use context_creator::core::telemetry::{JsonParser, OtlpParser};

    // Given: Valid OTLP JSON export with traces
    let otlp_json = r#"{
        "resourceSpans": [{
            "resource": {
                "attributes": [{
                    "key": "service.name",
                    "value": { "stringValue": "payment-api" }
                }]
            },
            "scopeSpans": [{
                "spans": [{
                    "name": "process_payment",
                    "startTimeUnixNano": "1704067200000000000",
                    "endTimeUnixNano": "1704067200050000000",
                    "attributes": [
                        {
                            "key": "code.function.name",
                            "value": { "stringValue": "process_payment" }
                        },
                        {
                            "key": "code.file.path",
                            "value": { "stringValue": "src/api/handlers.rs" }
                        },
                        {
                            "key": "code.line.number",
                            "value": { "intValue": "42" }
                        }
                    ]
                }]
            }]
        }]
    }"#;

    // When: Parsing the file
    let parser = JsonParser::new();
    let result = parser.parse_bytes(otlp_json.as_bytes()).unwrap();

    // Then: Should extract trace data with code attributes
    assert_eq!(result.spans.len(), 1);
    assert_eq!(result.code_spans.len(), 1);

    let span = &result.code_spans[0];
    assert_eq!(span.name, "process_payment");
    assert_eq!(span.function_name.as_deref(), Some("process_payment"));
    assert_eq!(span.service_name.as_deref(), Some("payment-api"));
}

#[test]
fn test_parse_otlp_protobuf_format() {
    use context_creator::core::telemetry::otlp_parser::ProtobufParser;
    use context_creator::core::telemetry::OtlpParser;

    let parser = ProtobufParser::new();
    let result = parser.parse_bytes(&valid_protobuf_trace_bytes()).unwrap();

    assert_eq!(result.spans.len(), 1);
    assert_eq!(result.code_spans.len(), 1);

    let span = &result.code_spans[0];
    assert_eq!(span.name, "process_payment");
    assert_eq!(span.function_name.as_deref(), Some("process_payment"));
    assert_eq!(span.service_name.as_deref(), Some("payment-api"));
    assert_eq!(span.file_path, Some(PathBuf::from("src/api/handlers.rs")));
    assert_eq!(span.line_number, Some(1));
}

#[test]
fn test_telemetry_command_with_valid_protobuf_file() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("src/api");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(
        source_dir.join("handlers.rs"),
        r#"pub fn process_payment() {
    println!("paid");
}
"#,
    )
    .unwrap();

    let telemetry_file = temp_dir.path().join("traces.pb");
    write_valid_protobuf_trace(&telemetry_file);

    let mut cmd = assert_cmd::Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .arg("telemetry")
        .arg("-t")
        .arg(&telemetry_file)
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "telemetry protobuf command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Total spans: 1"), "{stdout}");
    assert!(stdout.contains("Spans after filters: 1"), "{stdout}");
    assert!(stdout.contains("Correlated spans: 1"), "{stdout}");
    assert!(stdout.contains("process_payment"), "{stdout}");
}

#[test]
fn test_correlate_trace_to_function() {
    use context_creator::core::semantic::analyzer::FunctionDefinition;
    use context_creator::core::semantic::dependency_types::FileAnalysisResult;
    use context_creator::core::telemetry::{TelemetryCorrelator, TelemetrySpan};
    use std::collections::HashMap;

    // Given: Trace with code.function.name = "process_payment"
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

    // And: Analysis result with matching function
    let analysis_result = FileAnalysisResult {
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

    // When: Correlating telemetry to code
    let correlator =
        TelemetryCorrelator::new(vec![analysis_result], file_paths, PathBuf::from("/project"));
    let result = correlator.correlate_spans(vec![span]);

    // Then: Should match trace to correct function
    assert_eq!(result.correlated_count, 1);
    assert_eq!(result.function_metrics.len(), 1);

    let key = result.function_metrics.keys().next().unwrap();
    assert_eq!(key.function_name, "process_payment");
    assert_eq!(key.file_path, PathBuf::from("/project/src/api/handlers.rs"));
}

#[test]
fn test_enrich_function_with_metrics() {
    use context_creator::core::telemetry::{
        CorrelationKey, CorrelationResult, FunctionMetrics, TelemetryEnricher,
    };
    use std::collections::HashMap;

    // Given: Function with associated traces showing p95 latency
    let mut function_metrics = HashMap::new();

    let key = CorrelationKey {
        file_path: PathBuf::from("/project/src/api/handlers.rs"),
        function_name: "process_payment".to_string(),
        line_number: Some(42),
    };

    let mut metrics = FunctionMetrics::new();
    metrics.call_count = 5;
    // Use add_latency to populate data properly
    for latency in [45.0, 50.0, 120.0, 130.0, 250.0] {
        metrics.add_latency(latency);
    }

    function_metrics.insert(key.clone(), metrics);

    let correlation_result = CorrelationResult {
        function_metrics,
        correlated_count: 5,
        uncorrelated_spans: vec![],
    };

    // When: Enriching source code
    let enricher = TelemetryEnricher::new(correlation_result);
    let comment = enricher.generate_function_comment(&key).unwrap();

    // Then: Should add latency metrics to function header comment
    assert!(comment.contains("<!-- OpenTelemetry Metrics"));
    assert!(comment.contains("Calls: 5"));
    assert!(comment.contains("p50=120ms")); // Fixed: p50 of [45, 50, 120, 130, 250] is 120
    assert!(comment.contains("p95=250ms")); // Fixed: p95 of 5 values is the 5th value
    assert!(comment.contains("p99=250ms")); // Fixed: p99 of 5 values is the 5th value
    assert!(comment.contains("-->"));
}

#[test]
fn test_handle_missing_code_attributes() {
    use context_creator::core::telemetry::{JsonParser, OtlpParser};

    // Given: Telemetry data without code attributes
    let otlp_json = r#"{
        "resourceSpans": [{
            "scopeSpans": [{
                "spans": [{
                    "name": "database_query",
                    "startTimeUnixNano": "1704067200000000000",
                    "endTimeUnixNano": "1704067200100000000",
                    "attributes": []
                }]
            }]
        }]
    }"#;

    // When: Processing correlation
    let parser = JsonParser::new();
    let result = parser.parse_bytes(otlp_json.as_bytes()).unwrap();

    // Then: Should gracefully skip without errors
    assert_eq!(result.spans.len(), 1);
    assert_eq!(result.code_spans.len(), 0); // No code attributes, so not included
    assert_eq!(result.spans[0].name, "database_query");
}

#[test]
fn test_integration_otlp_to_enriched_output() {
    use assert_cmd::Command;
    use predicates::prelude::*;

    // Given: OTLP file and matching source code
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("src/main.rs");
    fs::create_dir_all(source_file.parent().unwrap()).unwrap();
    fs::write(
        &source_file,
        r#"
fn calculate_total(items: Vec<f64>) -> f64 {
    items.iter().sum()
}

fn main() {
    let result = calculate_total(vec![1.0, 2.0, 3.0]);
    println!("Total: {}", result);
}
"#,
    )
    .unwrap();

    // Create OTLP telemetry file with multiple calls
    let telemetry_file = temp_dir.path().join("traces.json");
    fs::write(
        &telemetry_file,
        r#"{
        "resourceSpans": [{
            "resource": {
                "attributes": [{
                    "key": "service.name",
                    "value": { "stringValue": "calculator-service" }
                }]
            },
            "scopeSpans": [{
                "spans": [
                    {
                        "name": "calculate_total",
                        "startTimeUnixNano": "1704067200000000000",
                        "endTimeUnixNano": "1704067200050000000",
                        "attributes": [
                            {
                                "key": "code.function.name",
                                "value": { "stringValue": "calculate_total" }
                            },
                            {
                                "key": "code.file.path",
                                "value": { "stringValue": "src/main.rs" }
                            }
                        ]
                    },
                    {
                        "name": "calculate_total",
                        "startTimeUnixNano": "1704067201000000000",
                        "endTimeUnixNano": "1704067201045000000",
                        "attributes": [
                            {
                                "key": "code.function.name",
                                "value": { "stringValue": "calculate_total" }
                            },
                            {
                                "key": "code.file.path",
                                "value": { "stringValue": "src/main.rs" }
                            }
                        ]
                    },
                    {
                        "name": "calculate_total",
                        "startTimeUnixNano": "1704067202000000000",
                        "endTimeUnixNano": "1704067202055000000",
                        "attributes": [
                            {
                                "key": "code.function.name",
                                "value": { "stringValue": "calculate_total" }
                            },
                            {
                                "key": "code.file.path",
                                "value": { "stringValue": "src/main.rs" }
                            }
                        ]
                    }
                ]
            }]
        }]
    }"#,
    )
    .unwrap();

    // When: Running telemetry command
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let assert = cmd
        .arg("telemetry")
        .arg("-t")
        .arg(&telemetry_file)
        .arg(temp_dir.path())
        .assert();

    // Then: Output should contain correlation summary and metrics
    assert
        .success()
        .stdout(predicate::str::contains("Telemetry Correlation Summary"))
        .stdout(predicate::str::contains("Total spans: 3"))
        .stdout(predicate::str::contains("Correlated spans: 3"))
        .stdout(predicate::str::contains("Functions with metrics: 1"))
        .stdout(predicate::str::contains("# OpenTelemetry Metrics Summary"))
        .stdout(predicate::str::contains("### Function: calculate_total"))
        .stdout(predicate::str::contains("Calls: 3"))
        .stdout(predicate::str::contains("Latency p50:"));
}
