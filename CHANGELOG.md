# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Initial release of code-digest
- Core functionality for directory traversal
- Markdown generation from source code
- Token counting with tiktoken-rs
- File prioritization system
- Integration with gemini
- Support for .gitignore and .digestignore
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

[Unreleased]: https://github.com/matiasvillaverde/code-digest/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/matiasvillaverde/code-digest/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/matiasvillaverde/code-digest/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/matiasvillaverde/code-digest/releases/tag/v0.1.0