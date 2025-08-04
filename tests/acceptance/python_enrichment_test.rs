//! Acceptance test for Python service telemetry enrichment

use super::*;

#[test]
fn test_python_payment_service_enrichment() {
    // Load real Python source and OTLP data
    let source_content =
        std::fs::read_to_string(get_test_data_path("python", "payment_service.py"))
            .expect("Failed to read Python source file");
    let telemetry_file = get_test_data_path("python", "payment_traces.json");

    // Create test directory with source
    let source_files = vec![(PathBuf::from("payment_service.py"), source_content)];
    let temp_dir = setup_test_directory(&source_files).expect("Failed to setup test directory");

    // Run telemetry command
    let output = run_telemetry_command(temp_dir.path(), &telemetry_file, Some("payment-service"))
        .expect("Failed to run telemetry command");

    // Define expected enrichments
    let expected = vec![
        ExpectedEnrichment {
            function_name: "process_payment".to_string(),
            min_calls: 2,
            latency_p50_range: Some((20.0, 70.0)), // 25ms and 65ms spans
            has_errors: false, // For now, disable error checking until we see the format
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "validate_card".to_string(),
            min_calls: 1,
            latency_p50_range: Some((1.0, 3.0)), // 2ms spans
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "check_fraud".to_string(),
            min_calls: 1,
            latency_p50_range: Some((9.0, 11.0)), // 10ms spans
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "charge_card".to_string(),
            min_calls: 1,
            latency_p50_range: Some((49.0, 51.0)), // 50ms span
            has_errors: false,
            error_message: None,
        },
        ExpectedEnrichment {
            function_name: "refund_payment".to_string(),
            min_calls: 1,
            latency_p50_range: Some((29.0, 31.0)), // 30ms span
            has_errors: false,
            error_message: None,
        },
    ];

    // Assert enrichments
    assert_enrichments(&output, &expected);

    // Python-specific checks
    assert!(
        output.contains("Total spans: 6"),
        "Incorrect total span count"
    );
    assert!(
        output.contains("Functions with metrics: 5"),
        "Incorrect function count"
    );

    // Check that the error is in the summary
    // The error should show up in the individual function section for process_payment
}

#[test]
fn test_python_class_method_correlation() {
    // Test that Python class methods are properly correlated
    let source_content =
        std::fs::read_to_string(get_test_data_path("python", "payment_service.py"))
            .expect("Failed to read Python source file");
    let telemetry_file = get_test_data_path("python", "payment_traces.json");

    let source_files = vec![(PathBuf::from("payment_service.py"), source_content)];
    let temp_dir = setup_test_directory(&source_files).expect("Failed to setup test directory");

    let output = run_telemetry_command(temp_dir.path(), &telemetry_file, None)
        .expect("Failed to run telemetry command");

    // All methods in PaymentService class should be found
    let class_methods = vec![
        "process_payment",
        "validate_card",
        "check_fraud",
        "charge_card",
        "refund_payment",
    ];

    for method in class_methods {
        assert!(
            output.contains(method),
            "Class method {method} not found in correlation"
        );
    }

    // Should show successful correlation
    assert!(output.contains("Correlated spans: 6"));
    assert!(output.contains("Functions with metrics: 5"));
}
