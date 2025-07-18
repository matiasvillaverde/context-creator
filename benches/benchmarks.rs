//! Performance benchmarks for context-creator
//!
//! This benchmark suite measures the performance of key components:
//! - Directory walking and file discovery
//! - File prioritization and token counting
//! - Markdown generation
//! - End-to-end processing performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use context_creator::core::{
    cache::FileCache,
    context_builder::{generate_markdown, ContextOptions},
    prioritizer::prioritize_files,
    token::TokenCounter,
    walker::{walk_directory, WalkOptions},
};
use context_creator::utils::file_ext::FileType;
use std::sync::Arc;

/// Create a test project with various file sizes and types
fn create_test_project(
    temp_dir: &Path,
    file_count: usize,
    avg_file_size: usize,
) -> std::path::PathBuf {
    let project_dir = temp_dir.join("benchmark_project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create nested directory structure
    fs::create_dir_all(project_dir.join("src/core")).unwrap();
    fs::create_dir_all(project_dir.join("src/utils")).unwrap();
    fs::create_dir_all(project_dir.join("tests")).unwrap();
    fs::create_dir_all(project_dir.join("benches")).unwrap();
    fs::create_dir_all(project_dir.join("examples")).unwrap();

    // Create various file types with different sizes
    let file_templates = [
        ("src/main.rs", "rust", "fn main() {\n    println!(\"Hello, world!\");\n}\n"),
        ("src/lib.rs", "rust", "//! Library module\n\npub mod core;\npub mod utils;\n"),
        ("src/core/mod.rs", "rust", "//! Core functionality\n\npub mod processor;\npub mod walker;\n"),
        ("src/utils/helpers.rs", "rust", "//! Utility functions\n\npub fn helper() -> String {\n    \"helper\".to_string()\n}\n"),
        ("tests/integration.rs", "rust", "#[cfg(test)]\nmod tests {\n    #[test]\n    fn test_basic() {\n        assert_eq!(1, 1);\n    }\n}\n"),
        ("README.md", "markdown", "# Benchmark Project\n\nThis is a test project for benchmarking.\n"),
        ("Cargo.toml", "toml", "[package]\nname = \"benchmark\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        ("package.json", "json", "{\n  \"name\": \"benchmark\",\n  \"version\": \"1.0.0\",\n  \"main\": \"index.js\"\n}\n"),
        ("config.yaml", "yaml", "database:\n  host: localhost\n  port: 5432\n  name: benchmark\n"),
        ("style.css", "css", "body {\n  font-family: Arial, sans-serif;\n  margin: 0;\n  padding: 20px;\n}\n"),
    ];

    // Create files up to the specified count
    for i in 0..file_count {
        let template_idx = i % file_templates.len();
        let (base_path, _, base_content) = &file_templates[template_idx];

        // Create unique filename
        let file_path = if i < file_templates.len() {
            project_dir.join(base_path)
        } else {
            project_dir.join(format!("src/generated_{i}.rs"))
        };

        // Scale content to target size
        let mut content = base_content.to_string();
        while content.len() < avg_file_size {
            content.push_str("\n// Additional content to reach target size");
            content.push_str(&format!("\n// Line {}", content.lines().count()));
        }

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        fs::write(&file_path, content).unwrap();
    }

    project_dir
}

/// Benchmark directory walking performance
fn bench_directory_walking(c: &mut Criterion) {
    let mut group = c.benchmark_group("directory_walking");

    for &file_count in &[10, 50, 100, 500] {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = create_test_project(temp_dir.path(), file_count, 1000);

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("walk_directory", file_count),
            &project_dir,
            |b, path| {
                let walk_options = WalkOptions::default();
                b.iter(|| {
                    black_box(
                        walk_directory(black_box(path), black_box(walk_options.clone())).unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark token counting performance
fn bench_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_counting");
    let token_counter = TokenCounter::new().unwrap();

    for &file_size in &[1000, 5000, 10_000, 50_000] {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = create_test_project(temp_dir.path(), 10, file_size);
        let walk_options = WalkOptions::default();
        let files = walk_directory(&project_dir, walk_options).unwrap();

        group.throughput(Throughput::Bytes(file_size as u64 * 10));
        group.bench_with_input(
            BenchmarkId::new("count_tokens", file_size),
            &files,
            |b, files| {
                b.iter(|| {
                    for file in files {
                        if let Ok(content) = fs::read_to_string(&file.path) {
                            black_box(token_counter.count_tokens(black_box(&content)).unwrap());
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file prioritization performance
fn bench_file_prioritization(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_prioritization");

    for &file_count in &[50, 100, 500, 1000] {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = create_test_project(temp_dir.path(), file_count, 2000);
        let walk_options = WalkOptions::default();
        let files = walk_directory(&project_dir, walk_options).unwrap();

        let context_options = ContextOptions {
            max_tokens: Some(50_000),
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context".to_string(),
            include_toc: true,
            enhanced_context: false,
        };

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("prioritize_files", file_count),
            &(&files, &context_options),
            |b, (files, options)| {
                b.iter(|| {
                    let files_clone = (*files).clone();
                    let options_clone = (*options).clone();
                    let cache = Arc::new(FileCache::new());
                    black_box(prioritize_files(files_clone, &options_clone, cache).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark markdown generation performance
fn bench_markdown_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("markdown_generation");

    for &file_count in &[10, 50, 100, 200] {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = create_test_project(temp_dir.path(), file_count, 3000);
        let walk_options = WalkOptions::default();
        let files = walk_directory(&project_dir, walk_options).unwrap();

        let context_options = ContextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context".to_string(),
            include_toc: true,
            enhanced_context: false,
        };

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("generate_markdown", file_count),
            &(&files, &context_options),
            |b, (files, options)| {
                b.iter(|| {
                    let files_clone = (*files).clone();
                    let options_clone = (*options).clone();
                    let cache = Arc::new(FileCache::new());
                    // Pre-populate cache with file contents
                    for file in &files_clone {
                        fs::write(&file.path, "test content").ok();
                    }
                    black_box(generate_markdown(files_clone, options_clone, cache).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark end-to-end processing performance
fn bench_end_to_end_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    for &file_count in &[25, 50, 100, 200, 1000] {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = create_test_project(temp_dir.path(), file_count, 2500);

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("full_processing", file_count),
            &project_dir,
            |b, path| {
                b.iter(|| {
                    // Complete end-to-end processing pipeline
                    let walk_options = WalkOptions::default();
                    let files = walk_directory(black_box(path), walk_options).unwrap();

                    let context_options = ContextOptions {
                        max_tokens: Some(100_000),
                        include_tree: true,
                        include_stats: true,
                        group_by_type: false,
                        sort_by_priority: true,
                        file_header_template: "## {path}".to_string(),
                        doc_header_template: "# Code Context".to_string(),
                        include_toc: true,
                        enhanced_context: false,
                    };

                    let cache = Arc::new(FileCache::new());
                    let prioritized_files =
                        prioritize_files(files, &context_options, cache.clone()).unwrap();
                    let _markdown =
                        generate_markdown(prioritized_files, context_options, cache).unwrap();

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    // Test with progressively larger projects
    for &(file_count, avg_size) in &[(100, 1000), (200, 2000), (500, 1500), (1000, 1000)] {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = create_test_project(temp_dir.path(), file_count, avg_size);

        let total_bytes = file_count * avg_size;
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(
            BenchmarkId::new("memory_usage", format!("{file_count}files_{avg_size}bytes")),
            &project_dir,
            |b, path| {
                b.iter(|| {
                    let walk_options = WalkOptions::default();
                    let files = walk_directory(black_box(path), walk_options).unwrap();

                    // Simulate memory-intensive operations
                    let mut all_content = String::new();
                    for file in &files {
                        if let Ok(content) = fs::read_to_string(&file.path) {
                            all_content.push_str(&content);
                        }
                    }

                    black_box(all_content);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel processing performance
fn bench_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");
    let token_counter = TokenCounter::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_test_project(temp_dir.path(), 200, 5000);
    let walk_options = WalkOptions::default();
    let files = walk_directory(&project_dir, walk_options).unwrap();

    group.bench_function("sequential_token_counting", |b| {
        b.iter(|| {
            for file in &files {
                if let Ok(content) = fs::read_to_string(&file.path) {
                    black_box(token_counter.count_tokens(black_box(&content)).unwrap());
                }
            }
        });
    });

    group.bench_function("parallel_token_counting", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let _: Vec<_> = files
                .par_iter()
                .filter_map(|file| fs::read_to_string(&file.path).ok())
                .map(|content| token_counter.count_tokens(&content).unwrap())
                .collect();
            black_box(());
        });
    });

    group.finish();
}

/// Benchmark different file type processing
fn bench_file_type_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_type_processing");
    let token_counter = TokenCounter::new().unwrap();

    let temp_dir = TempDir::new().unwrap();

    // Create files of different types
    let file_types = [
        ("large.rs", FileType::Rust, "fn main() {\n    // Rust code\n    println!(\"Hello\");\n}\n".repeat(100)),
        ("large.py", FileType::Python, "# Python code\ndef main():\n    print(\"Hello\")\n\nif __name__ == \"__main__\":\n    main()\n".repeat(100)),
        ("large.js", FileType::JavaScript, "// JavaScript code\nfunction main() {\n    console.log(\"Hello\");\n}\nmain();\n".repeat(100)),
        ("large.md", FileType::Markdown, "# Markdown\n\nThis is **markdown** content.\n\n- List item 1\n- List item 2\n".repeat(100)),
        ("large.json", FileType::Json, "{\n  \"key\": \"value\",\n  \"number\": 42,\n  \"array\": [1, 2, 3]\n}\n".repeat(50)),
    ];

    for (filename, file_type, content) in &file_types {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).unwrap();

        group.bench_with_input(
            BenchmarkId::new("token_counting", format!("{file_type:?}")),
            &(file_path, content),
            |b, (_path, content)| {
                b.iter(|| {
                    black_box(token_counter.count_tokens(black_box(content)).unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_directory_walking,
    bench_token_counting,
    bench_file_prioritization,
    bench_markdown_generation,
    bench_end_to_end_processing,
    bench_memory_efficiency,
    bench_parallel_processing,
    bench_file_type_processing
);

criterion_main!(benches);
