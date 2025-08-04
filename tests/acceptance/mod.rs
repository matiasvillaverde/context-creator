//! Acceptance tests for telemetry command with real-world OpenTelemetry data

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

mod python_enrichment_test;
mod rust_enrichment_test;
mod typescript_enrichment_test;

/// Structure to define expected enrichment in test output
#[derive(Debug)]
pub struct ExpectedEnrichment {
    pub function_name: String,
    pub min_calls: usize,
    pub latency_p50_range: Option<(f64, f64)>,
    pub has_errors: bool,
    pub error_message: Option<String>,
}

/// Helper to run telemetry command and return output
pub fn run_telemetry_command(
    source_dir: &Path,
    telemetry_file: &Path,
    service_filter: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("context-creator")?;

    cmd.arg("telemetry").arg("-t").arg(telemetry_file);

    if let Some(service) = service_filter {
        cmd.arg("--service").arg(service);
    }

    cmd.arg(source_dir);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(format!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Helper to create a temporary directory with source files
pub fn setup_test_directory(
    source_files: &[(PathBuf, String)],
) -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    for (relative_path, content) in source_files {
        let full_path = temp_dir.path().join(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full_path, content)?;
    }

    Ok(temp_dir)
}

/// Assert that the output contains expected enrichments
pub fn assert_enrichments(output: &str, expected: &[ExpectedEnrichment]) {
    // Check correlation summary
    assert!(
        output.contains("Telemetry Correlation Summary:"),
        "Missing correlation summary"
    );

    // Check each expected enrichment
    for enrichment in expected {
        // Function should be mentioned in the summary
        assert!(
            output.contains(&enrichment.function_name),
            "Function {} not found in output",
            enrichment.function_name
        );

        // Check call count
        if enrichment.min_calls > 0 {
            let calls_pattern = format!("Calls: {}", enrichment.min_calls);
            assert!(
                output.contains(&calls_pattern)
                    || output.contains(&format!("Calls: {}", enrichment.min_calls + 1))
                    || output.contains(&format!("Calls: {}", enrichment.min_calls + 2)),
                "Expected at least {} calls for {}, but pattern not found",
                enrichment.min_calls,
                enrichment.function_name
            );
        }

        // Check latency ranges if specified
        if let Some((min_p50, max_p50)) = enrichment.latency_p50_range {
            assert!(
                output.contains("Latency p50:"),
                "Missing latency metrics for {}",
                enrichment.function_name
            );

            // Extract p50 value and verify it's in range
            // Look for "Latency p50: XXms" format
            let p50_present = output.lines().any(|line| {
                if line.contains("Latency p50:") {
                    if let Some(p50_str) = line.split("Latency p50:").nth(1) {
                        if let Some(ms_str) = p50_str.split("ms").next() {
                            if let Ok(p50_val) = ms_str.trim().parse::<f64>() {
                                return p50_val >= min_p50 && p50_val <= max_p50;
                            }
                        }
                    }
                }
                false
            });

            assert!(
                p50_present,
                "p50 latency for {} not in expected range {}-{}ms",
                enrichment.function_name, min_p50, max_p50
            );
        }

        // Check error presence
        if enrichment.has_errors {
            assert!(
                output.contains("Error Rate:")
                    || output.contains("error")
                    || output.contains("ERROR"),
                "Expected errors for {} but none found",
                enrichment.function_name
            );

            if let Some(ref error_msg) = enrichment.error_message {
                assert!(
                    output.contains(error_msg),
                    "Expected error message '{error_msg}' not found"
                );
            }
        }
    }
}

/// Helper to get test data path
pub fn get_test_data_path(language: &str, filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("acceptance")
        .join(language)
        .join(filename)
}
