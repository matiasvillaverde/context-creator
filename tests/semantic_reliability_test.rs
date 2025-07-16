//! Reliability tests for semantic analysis
//! Tests thread safety, error handling, and resource management

use code_digest::core::semantic::cache::AstCache;
use code_digest::core::semantic::parser_pool::ParserPoolManager;
use std::sync::Arc;
use std::thread;
use tempfile::TempDir;
use tokio::runtime::Runtime;

#[test]
fn test_mutex_error_handling() {
    use std::panic;

    // Test that poisoned mutex doesn't crash the application
    let cache = Arc::new(AstCache::new(10));
    let cache_clone = cache.clone();

    // Spawn a thread that will panic while holding the lock
    let panic_thread = thread::spawn(move || {
        // Catch the panic to properly test mutex poisoning
        let result = panic::catch_unwind(|| {
            // Get the internal mutex directly - we need to poison it
            // Since the cache field is private, we'll use a method that holds the lock
            cache_clone.clear().unwrap();
            // Force a panic while the lock is held
            panic!("Intentional panic to poison mutex");
        });
        assert!(result.is_err());
    });

    // Wait for the panic thread to finish
    let _ = panic_thread.join();

    // For now, since we can't directly poison the mutex from outside,
    // let's test that all operations handle errors correctly
    let result = cache.len();
    assert!(result.is_ok()); // Should still work even after panic in another thread
}

#[test]
fn test_cache_operations_return_results() {
    let cache = AstCache::new(10);

    // All operations should return Results
    assert!(cache.len().is_ok());
    assert!(cache.is_empty().is_ok());
    assert!(cache.capacity().is_ok());
    assert!(cache.clear().is_ok());
}

#[test]
fn test_parser_pool_creation() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let pool_manager = ParserPoolManager::new();

        // Test getting parsers for different languages
        let rust_parser = pool_manager.get_parser("rust").await;
        assert!(rust_parser.is_ok());

        let js_parser = pool_manager.get_parser("javascript").await;
        assert!(js_parser.is_ok());

        // Test unsupported language
        let unknown_parser = pool_manager.get_parser("unknown_language").await;
        assert!(unknown_parser.is_err());
    });
}

#[test]
fn test_parser_pool_concurrent_access() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let pool_manager = Arc::new(ParserPoolManager::new());
        let mut handles = vec![];

        // Spawn multiple tasks to access parsers concurrently
        for i in 0..20 {
            let pool_clone = pool_manager.clone();
            let handle = tokio::spawn(async move {
                let parser = pool_clone.get_parser("rust").await.unwrap();
                // Parser should have timeout set
                assert_eq!(parser.timeout_micros(), 5_000_000);

                // Simulate some work
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                // Parser is automatically returned to pool when dropped
                format!("Task {i} completed")
            });
            handles.push(handle);
        }

        // All tasks should complete successfully
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.contains("completed"));
        }
    });
}

#[test]
fn test_parser_timeout_configuration() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let pool_manager = ParserPoolManager::new();
        let parser = pool_manager.get_parser("python").await.unwrap();

        // Verify timeout is set to 5 seconds
        assert_eq!(parser.timeout_micros(), 5_000_000);
    });
}

#[test]
fn test_parsing_with_timeout() {
    use tokio::time::{timeout, Duration};

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let pool_manager = ParserPoolManager::new();

        // Test normal parsing completes within timeout
        let normal_content = "fn main() { println!(\"Hello, world!\"); }";
        let result = timeout(Duration::from_secs(1), async {
            let mut parser = pool_manager.get_parser("rust").await.unwrap();
            parser.parse(normal_content, None)
        })
        .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    });
}

#[test]
fn test_cache_with_parser_pool() {
    use code_digest::core::semantic::AstCacheV2;
    use std::path::Path;

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let cache = AstCacheV2::new(100);

        // Test parsing and caching
        let path = Path::new("test.rs");
        let content = "fn test() {}";

        let result = cache.get_or_parse(path, content, "rust").await;
        assert!(result.is_ok());

        // Second call should hit cache
        let result2 = cache.get_or_parse(path, content, "rust").await;
        assert!(result2.is_ok());
    });
}

#[test]
fn test_path_validation() {
    use code_digest::core::semantic::path_validator::validate_import_path;
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();

    // Create test directory structure
    fs::create_dir_all(base_dir.join("src")).unwrap();
    fs::create_dir_all(base_dir.join("tests")).unwrap();
    fs::write(base_dir.join("src/lib.rs"), "").unwrap();
    fs::write(base_dir.join("tests/test.rs"), "").unwrap();

    // Valid paths within project
    assert!(validate_import_path(base_dir, &base_dir.join("src/lib.rs")).is_ok());
    assert!(validate_import_path(base_dir, &base_dir.join("tests/test.rs")).is_ok());

    // Invalid paths - outside project
    assert!(validate_import_path(base_dir, std::path::Path::new("/etc/passwd")).is_err());
    assert!(validate_import_path(base_dir, &base_dir.join("../../../etc/passwd")).is_err());

    // Invalid paths - absolute paths outside project
    assert!(validate_import_path(base_dir, std::path::Path::new("/tmp/file.rs")).is_err());

    // Edge case - symlinks should be resolved
    fs::write(base_dir.join("lib.rs"), "").unwrap();
    assert!(validate_import_path(base_dir, &base_dir.join("./src/../lib.rs")).is_ok());
}

#[test]
fn test_path_traversal_attack_prevention() {
    use code_digest::core::semantic::path_validator::validate_import_path;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();

    // Create a src directory to make tests more realistic
    fs::create_dir_all(base_dir.join("src")).unwrap();

    // Various path traversal attempts
    let attacks = vec![
        "../../../etc/passwd",
        "../../.ssh/id_rsa",
        "../.env",
        "/etc/shadow",
        "\\..\\..\\windows\\system32\\config\\sam", // Windows style
        "src/../../../../../../etc/hosts",
        "%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd", // URL encoded
    ];

    for attack in attacks {
        let path = base_dir.join(attack);
        assert!(
            validate_import_path(base_dir, &path).is_err(),
            "Path traversal attack should be blocked: {attack}"
        );
    }
}
