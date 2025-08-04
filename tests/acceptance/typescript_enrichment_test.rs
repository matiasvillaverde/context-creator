//! Acceptance test for TypeScript service telemetry enrichment

use super::*;

#[test]
fn test_typescript_frontend_service_enrichment() {
    // Load real TypeScript source and OTLP data
    let source_content = std::fs::read_to_string(get_test_data_path("typescript", "frontend.ts"))
        .expect("Failed to read TypeScript source file");
    let telemetry_file = get_test_data_path("typescript", "frontend_traces.json");

    // Create test directory with source
    let source_files = vec![(PathBuf::from("frontend.ts"), source_content)];
    let temp_dir = setup_test_directory(&source_files).expect("Failed to setup test directory");

    // Run telemetry command
    let output = run_telemetry_command(temp_dir.path(), &telemetry_file, Some("frontend-service"))
        .expect("Failed to run telemetry command");

    // Define expected enrichments
    let expected = vec![
        ExpectedEnrichment {
            function_name: "getProducts".to_string(),
            min_calls: 1,
            latency_p50_range: Some((44.0, 46.0)), // 45ms span
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "getProduct".to_string(),
            min_calls: 1,
            latency_p50_range: Some((24.0, 26.0)), // 25ms span
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "addToCart".to_string(),
            min_calls: 1,
            latency_p50_range: Some((34.0, 36.0)), // 35ms span
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "getCart".to_string(),
            min_calls: 2, // Called directly once, and once from checkout
            latency_p50_range: Some((10.0, 20.0)), // 10ms and 20ms spans
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "checkout".to_string(),
            min_calls: 1,
            latency_p50_range: Some((119.0, 121.0)), // 120ms span
            has_errors: false,
            error_message: None,
        },
    ];

    // Assert enrichments
    assert_enrichments(&output, &expected);

    // TypeScript-specific checks
    assert!(
        output.contains("Total spans: 6"),
        "Incorrect total span count"
    );
    assert!(
        output.contains("Functions with metrics: 5"),
        "Incorrect function count"
    );
}

#[test]
fn test_typescript_async_method_correlation() {
    // Test that async TypeScript methods are properly correlated
    let source_content = std::fs::read_to_string(get_test_data_path("typescript", "frontend.ts"))
        .expect("Failed to read TypeScript source file");
    let telemetry_file = get_test_data_path("typescript", "frontend_traces.json");

    let source_files = vec![(PathBuf::from("frontend.ts"), source_content)];
    let temp_dir = setup_test_directory(&source_files).expect("Failed to setup test directory");

    let output = run_telemetry_command(temp_dir.path(), &telemetry_file, None)
        .expect("Failed to run telemetry command");

    // All async methods should be found
    let async_methods = vec![
        "getProducts",
        "getProduct",
        "addToCart",
        "getCart",
        "checkout",
    ];

    for method in async_methods {
        assert!(
            output.contains(method),
            "Async method {method} not found in correlation"
        );
    }
}

#[test]
fn test_typescript_nested_span_correlation() {
    // Test nested spans (getCart called from checkout)
    let source_content = std::fs::read_to_string(get_test_data_path("typescript", "frontend.ts"))
        .expect("Failed to read TypeScript source file");
    let telemetry_file = get_test_data_path("typescript", "frontend_traces.json");

    let source_files = vec![(PathBuf::from("frontend.ts"), source_content)];
    let temp_dir = setup_test_directory(&source_files).expect("Failed to setup test directory");

    let output = run_telemetry_command(temp_dir.path(), &telemetry_file, None)
        .expect("Failed to run telemetry command");

    // getCart should have 2 calls (one direct, one from checkout)
    assert!(
        output.contains("getCart") && output.contains("Calls: 2"),
        "Nested span correlation not working correctly"
    );

    // All spans should be correlated
    assert!(output.contains("Correlated spans: 6"));
    assert!(output.contains("Uncorrelated spans: 0"));
}

#[test]
fn test_typescript_class_export_handling() {
    // Test that exported class methods are handled correctly
    let source_content = std::fs::read_to_string(get_test_data_path("typescript", "frontend.ts"))
        .expect("Failed to read TypeScript source file");
    let telemetry_file = get_test_data_path("typescript", "frontend_traces.json");

    let source_files = vec![(PathBuf::from("frontend.ts"), source_content)];
    let temp_dir = setup_test_directory(&source_files).expect("Failed to setup test directory");

    let output = run_telemetry_command(temp_dir.path(), &telemetry_file, None)
        .expect("Failed to run telemetry command");

    // Check that FrontendService class methods are found
    assert!(output.contains("### Function: getProducts"));
    assert!(output.contains("### Function: checkout"));
}
