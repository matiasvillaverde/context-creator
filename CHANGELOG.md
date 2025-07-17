# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- FileCache for eliminating redundant I/O operations
- Parallel token counting for improved performance
- Memory optimization with pre-allocated string buffers
- Performance benchmark for 1000 files
- Integration test verifying <1 second processing for 1000 files

### Changed
- Optimized file reading to eliminate 2-3x redundant I/O operations
- Improved token counting with parallel processing using rayon
- Enhanced memory efficiency in markdown generation

### Performance
- Reduced processing time for 1000 files from >5 seconds to <100ms (50x improvement)
- Eliminated redundant file reads through shared caching
- Reduced memory allocations with pre-sized string buffers

## [0.3.0] - 2025-01-12

### Added
- New `--prompt` / `-p` flag for specifying prompts with spaces
- Support for reading prompts from stdin (via pipe or `--stdin` flag)
- Positional arguments for directories (can now use `context-creator dir1 dir2 dir3`)
- Automatic stdin detection when input is piped
- Improved backward compatibility for existing command patterns

### Changed
- Enhanced CLI argument parsing with more flexible prompt and directory specification
- Better separation between prompt and directory arguments

## [0.2.0] - 2025-01-11

### Added
- Support for processing multiple directories in a single command
- Enhanced output formatting for multiple directory results
- Improved Windows CI test compatibility

### Fixed
- Windows test failures due to executable mocking limitations
- Clippy warnings about unused variables on Windows platform

## [0.1.1] - 2024-12-14

### Added
- Initial release of context-creator
- Core functionality for directory traversal
- Markdown generation from source code
- Token counting with tiktoken-rs
- File prioritization system
- Integration with gemini
- Support for .gitignore and .contextignore
- Configuration file support
- CLI interface with comprehensive options
- Parallel processing for performance
- Binary file detection and skipping

### Security
- Path traversal protection
- Safe handling of symbolic links

## [0.1.0] - 2024-12-13

### Added
- First public release

[Unreleased]: https://github.com/matiasvillaverde/context-creator/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/matiasvillaverde/context-creator/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/matiasvillaverde/context-creator/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/matiasvillaverde/context-creator/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/matiasvillaverde/context-creator/releases/tag/v0.1.0