# Rust Performance Optimization Techniques for Code-Digest Tool

This guide compiles practical Rust performance optimization techniques specifically relevant to the code-digest tool's bottlenecks, based on the latest best practices from 2024-2025.

## 1. Eliminating Redundant I/O Operations

### The Problem
The code-digest tool currently reads files multiple times:
- Once during the walker phase for metadata
- Once during prioritization for token counting
- Once during markdown generation for content

### Solutions

#### Implement File Content Caching
```rust
use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap; // Thread-safe HashMap

pub struct FileCache {
    cache: DashMap<PathBuf, Arc<str>>,
}

impl FileCache {
    pub fn new() -> Self {
        FileCache {
            cache: DashMap::new(),
        }
    }

    pub fn get_or_load(&self, path: &Path) -> Result<Arc<str>> {
        if let Some(content) = self.cache.get(path) {
            return Ok(content.clone());
        }

        let content = fs::read_to_string(path)?;
        let arc_content: Arc<str> = Arc::from(content.as_str());
        self.cache.insert(path.to_path_buf(), arc_content.clone());
        Ok(arc_content)
    }
}
```

#### Use Buffered I/O
```rust
use std::io::{BufReader, Read};
use std::fs::File;

fn read_file_buffered(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(8192, file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    Ok(content)
}
```

#### Batch Metadata Operations
```rust
// Instead of individual metadata calls
let metadata = fs::metadata(&path)?;

// Use WalkDir's built-in metadata
for entry in WalkDir::new(root) {
    let entry = entry?;
    let metadata = entry.metadata()?; // Cached by WalkDir
}
```

## 2. Efficient Parallel Processing with Rayon

### Current Issues
- Walker collects all entries before parallel processing
- Token counting is sequential
- No parallel markdown generation

### Optimized Patterns

#### Stream-Based Parallel Processing
```rust
use rayon::prelude::*;
use rayon::iter::ParallelBridge;

pub fn walk_directory_streaming(root: &Path, options: WalkOptions) -> Result<Vec<FileInfo>> {
    let walker = WalkBuilder::new(root)
        .threads(num_cpus::get())
        .build_parallel();

    let (tx, rx) = crossbeam_channel::unbounded();
    
    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |entry| {
            if let Ok(entry) = entry {
                if !entry.path().is_dir() {
                    if let Some(file_info) = process_file(entry.path(), root, &options).ok().flatten() {
                        let _ = tx.send(file_info);
                    }
                }
            }
            WalkState::Continue
        })
    });

    drop(tx);
    Ok(rx.into_iter().collect())
}
```

#### Parallel Token Counting with Chunking
```rust
pub fn count_tokens_parallel(files: &[FileInfo], cache: &FileCache) -> Vec<TokenCount> {
    let token_counter = TokenCounter::new().unwrap();
    
    files.par_chunks(100) // Process in chunks for better cache locality
        .flat_map(|chunk| {
            chunk.iter().map(|file| {
                let content = cache.get_or_load(&file.path).ok()?;
                let tokens = token_counter.count_tokens(&content).ok()?;
                Some(TokenCount {
                    file: file.clone(),
                    count: tokens,
                })
            }).filter_map(|x| x)
        })
        .collect()
}
```

#### Parallel Markdown Generation
```rust
pub fn generate_markdown_parallel(files: Vec<FileInfo>, options: DigestOptions) -> Result<String> {
    let mut output = String::with_capacity(estimate_output_size(&files));
    
    // Generate header and stats serially
    append_header(&mut output, &options);
    append_stats(&mut output, &files, &options);
    
    // Generate file contents in parallel
    let file_contents: Vec<String> = files
        .par_iter()
        .map(|file| generate_file_section(file, &options))
        .collect();
    
    // Append in order
    for content in file_contents {
        output.push_str(&content);
    }
    
    Ok(output)
}
```

## 3. Memory Allocation Optimization for String Building

### Pre-allocation Strategies

#### Accurate Capacity Estimation
```rust
fn estimate_output_size(files: &[FileInfo]) -> usize {
    let header_size = 500; // Estimated header/stats size
    let toc_size = files.len() * 80; // ~80 chars per TOC entry
    let content_size: usize = files.iter()
        .map(|f| f.size as usize + 100) // File size + markdown overhead
        .sum();
    
    header_size + toc_size + content_size
}

pub fn generate_markdown_optimized(files: Vec<FileInfo>, options: DigestOptions) -> Result<String> {
    let capacity = estimate_output_size(&files);
    let mut output = String::with_capacity(capacity);
    
    // Use write! macro instead of format! to avoid intermediate allocations
    use std::fmt::Write;
    
    write!(&mut output, "{}\n\n", options.doc_header_template.replace("{directory}", "."))?;
    
    // Continue building output...
    Ok(output)
}
```

#### String Builder Pattern
```rust
pub struct MarkdownBuilder {
    buffer: String,
    estimated_size: usize,
}

impl MarkdownBuilder {
    pub fn with_capacity(capacity: usize) -> Self {
        MarkdownBuilder {
            buffer: String::with_capacity(capacity),
            estimated_size: capacity,
        }
    }
    
    pub fn append_section(&mut self, title: &str, content: &str) {
        // Check if we need to grow
        let needed = title.len() + content.len() + 10;
        if self.buffer.capacity() - self.buffer.len() < needed {
            self.buffer.reserve(needed * 2); // Double the needed space
        }
        
        use std::fmt::Write;
        write!(&mut self.buffer, "## {}\n\n{}\n\n", title, content).unwrap();
    }
    
    pub fn build(self) -> String {
        self.buffer
    }
}
```

## 4. Caching Strategies for File Content Reuse

### Multi-Level Caching Architecture
```rust
pub struct MultiLevelCache {
    // L1: Hot cache for frequently accessed files
    hot_cache: LruCache<PathBuf, Arc<str>>,
    // L2: Full cache for all files
    full_cache: DashMap<PathBuf, Arc<str>>,
    // Metrics for adaptive caching
    access_counts: DashMap<PathBuf, AtomicUsize>,
}

impl MultiLevelCache {
    pub fn new(hot_cache_size: usize) -> Self {
        MultiLevelCache {
            hot_cache: LruCache::new(hot_cache_size),
            full_cache: DashMap::new(),
            access_counts: DashMap::new(),
        }
    }
    
    pub fn get_or_load(&mut self, path: &Path) -> Result<Arc<str>> {
        // Track access
        self.access_counts
            .entry(path.to_path_buf())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
        
        // Check L1 cache
        if let Some(content) = self.hot_cache.get(path) {
            return Ok(content.clone());
        }
        
        // Check L2 cache
        if let Some(content) = self.full_cache.get(path) {
            // Promote to L1 if frequently accessed
            if self.is_hot(path) {
                self.hot_cache.put(path.to_path_buf(), content.clone());
            }
            return Ok(content.clone());
        }
        
        // Load from disk
        let content = fs::read_to_string(path)?;
        let arc_content: Arc<str> = Arc::from(content.as_str());
        
        self.full_cache.insert(path.to_path_buf(), arc_content.clone());
        Ok(arc_content)
    }
    
    fn is_hot(&self, path: &Path) -> bool {
        self.access_counts
            .get(path)
            .map(|count| count.load(Ordering::Relaxed) > 2)
            .unwrap_or(false)
    }
}
```

## 5. Arc<String> vs String vs &str Trade-offs

### Best Practices for File Content

#### Use Arc<str> for Shared Immutable Content
```rust
pub struct FileInfo {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub size: u64,
    pub file_type: FileType,
    pub priority: f32,
    pub content: Option<Arc<str>>, // Shared immutable content
}

// Efficient cloning - only increments reference count
impl Clone for FileInfo {
    fn clone(&self) -> Self {
        FileInfo {
            path: self.path.clone(),
            relative_path: self.relative_path.clone(),
            size: self.size,
            file_type: self.file_type.clone(),
            priority: self.priority,
            content: self.content.clone(), // Cheap Arc clone
        }
    }
}
```

#### String View Pattern for Temporary Operations
```rust
pub fn process_content<'a>(content: &'a Arc<str>) -> Vec<&'a str> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect()
}
```

## 6. Struct Design to Avoid Unnecessary Cloning

### Separate Metadata from Content
```rust
// Instead of a monolithic struct
pub struct FileData {
    metadata: FileMetadata,
    content: Arc<str>,
}

pub struct FileMetadata {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub size: u64,
    pub file_type: FileType,
    pub priority: f32,
}

// Process metadata without loading content
pub fn filter_by_priority(files: &[FileMetadata], min_priority: f32) -> Vec<&FileMetadata> {
    files.iter()
        .filter(|f| f.priority >= min_priority)
        .collect()
}
```

### Lazy Fields Pattern
```rust
use once_cell::sync::OnceCell;

pub struct LazyFileInfo {
    pub path: PathBuf,
    pub metadata: FileMetadata,
    content: OnceCell<Arc<str>>,
    tokens: OnceCell<usize>,
}

impl LazyFileInfo {
    pub fn content(&self) -> Result<&Arc<str>> {
        self.content.get_or_try_init(|| {
            let content = fs::read_to_string(&self.path)?;
            Ok(Arc::from(content.as_str()))
        })
    }
    
    pub fn token_count(&self) -> Result<usize> {
        self.tokens.get_or_try_init(|| {
            let content = self.content()?;
            let counter = TokenCounter::new()?;
            counter.count_tokens(content)
        })
    }
}
```

## 7. Performance Benchmarking with Criterion

### Comprehensive Benchmarking Setup
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

fn bench_file_reading_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_reading");
    
    // Configure for accurate measurements
    group.sample_size(10); // Reduce for expensive operations
    group.measurement_time(Duration::from_secs(10));
    
    let test_file = create_test_file(1_000_000); // 1MB file
    
    // Benchmark standard reading
    group.bench_function("standard_read", |b| {
        b.iter(|| {
            let content = fs::read_to_string(&test_file).unwrap();
            black_box(content);
        });
    });
    
    // Benchmark buffered reading
    group.bench_function("buffered_read", |b| {
        b.iter(|| {
            let content = read_file_buffered(&test_file).unwrap();
            black_box(content);
        });
    });
    
    // Benchmark cached reading
    let cache = FileCache::new();
    group.bench_function("cached_read", |b| {
        b.iter(|| {
            let content = cache.get_or_load(&test_file).unwrap();
            black_box(content);
        });
    });
    
    group.finish();
}

fn bench_parallel_vs_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallelism");
    
    for file_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*file_count as u64));
        
        let files = create_test_files(*file_count);
        
        group.bench_with_input(
            BenchmarkId::new("sequential", file_count),
            &files,
            |b, files| {
                b.iter(|| process_files_sequential(black_box(files)));
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("parallel", file_count),
            &files,
            |b, files| {
                b.iter(|| process_files_parallel(black_box(files)));
            },
        );
    }
    
    group.finish();
}

// Profile-guided optimization
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

criterion_group!(benches, bench_file_reading_strategies, bench_parallel_vs_sequential);
criterion_main!(benches);
```

### Cargo.toml Optimization Settings
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"

[profile.bench]
inherits = "release"
debug = true  # For profiling

[dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
rayon = "1.8"
dashmap = "5.5"
crossbeam-channel = "0.5"
once_cell = "1.19"
lru = "0.12"
tikv-jemallocator = { version = "0.5", optional = true }

[features]
jemalloc = ["tikv-jemallocator"]
```

## Summary

These optimization techniques focus on:
1. **Eliminating redundant I/O** through caching and buffered operations
2. **Efficient parallelization** using rayon's streaming APIs
3. **Smart memory allocation** with pre-allocation and capacity estimation
4. **Strategic caching** with multi-level cache architecture
5. **Using Arc<str>** for shared immutable file content
6. **Optimized struct design** with lazy fields and separated concerns
7. **Comprehensive benchmarking** with Criterion for validation

Each optimization targets specific bottlenecks identified in the code-digest tool, providing practical, measurable performance improvements while maintaining Rust's safety guarantees.