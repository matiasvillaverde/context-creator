# Parallel Processing in context-creator

This document describes the optimal parallel processing workflow implemented in context-creator's semantic analysis.

## Overview

The semantic analysis uses rayon for parallel processing to efficiently analyze large codebases. The workflow is designed to maximize parallelism while maintaining correctness and handling circular dependencies.

## Workflow Stages

### 1. Sequential File Discovery
- Walk the directory tree to discover all files
- Apply ignore patterns and filters
- Build initial file list

### 2. Parallel File Analysis
The core analysis happens in parallel using rayon's `par_iter()`:

```rust
let results: Vec<FileAnalysisResult> = (0..self.nodes.len())
    .into_par_iter()
    .map(|file_idx| {
        self.analyze_file_parallel(file_idx, options, cache, project_root)
    })
    .collect();
```

Each file is analyzed independently to extract:
- Import statements
- Function calls
- Type references
- Content hash for caching

### 3. Sequential Graph Building
After parallel analysis, the dependency graph is built sequentially:
- Resolve import paths
- Add typed edges (Import, FunctionCall, etc.)
- Build reverse dependencies

### 4. Cycle Detection
Using Kahn's algorithm (topological sort) to:
- Detect circular dependencies
- Determine safe processing order
- Handle cycles gracefully with warnings

## Key Features

### Thread-Safe Error Collection
Errors during parallel processing are collected safely:

```rust
let errors = Arc::new(Mutex::new(Vec::new()));
```

### Rich Edge Types
The graph uses typed edges to distinguish different relationships:
- `Import { symbols: Vec<String> }`
- `FunctionCall { function_name: String, module: Option<String> }`
- `TypeReference { type_name: String, is_generic: bool }`
- `Inheritance { base_type: String }`
- `InterfaceImplementation { interface_name: String }`

### Content Hashing
Each file's content is hashed during analysis for:
- Cache invalidation
- Change detection
- Incremental analysis (future feature)

## Performance Characteristics

From our tests with 50 interconnected files:
- Parallel analysis completes in ~400ms
- Linear scalability with number of CPU cores
- Minimal overhead from thread coordination

## Error Handling

The parallel workflow is resilient to errors:
- Syntax errors in one file don't break analysis of others
- Parse failures are logged but don't halt processing
- Results are always returned, even if partial

## Future Optimizations

Potential improvements:
1. Incremental analysis using content hashes
2. Parallel graph building for independent components
3. Work-stealing for better load balancing
4. Cache persistence between runs