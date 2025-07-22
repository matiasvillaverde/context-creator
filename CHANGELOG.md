# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2025-01-22

### Changed
- Renamed `--repo` flag to `--remote` for consistency with similar tools like repomix
- Improved flag naming conventions for better user experience

### Added
- Support for processing individual files (not just directories)
- Comprehensive edge case test suite covering various scenarios
- Enhanced error handling for symlinks and permission errors

### Fixed
- Graceful error handling without stack traces for better user experience
- Symlink and permission errors in CI environments

## [1.0.2] - 2025-07-21

### Performance
- Replaced O(n²) contains checks with HashSet for O(1) lookups
- Added semantic analysis caching to avoid redundant parsing
- Implemented single-pass directory walking for semantic analysis
- Removed debug eprintln statements for performance

### Fixed
- Restored backward compatibility for --include-callers when no context provided
- Resolved Python imports and Rust super:: imports in semantic analysis

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

[Unreleased]: https://github.com/matiasvillaverde/context-creator/compare/v1.0.2...HEAD
[1.0.2]: https://github.com/matiasvillaverde/context-creator/compare/v0.3.0...v1.0.2
[0.3.0]: https://github.com/matiasvillaverde/context-creator/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/matiasvillaverde/context-creator/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/matiasvillaverde/context-creator/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/matiasvillaverde/context-creator/releases/tag/v0.1.0