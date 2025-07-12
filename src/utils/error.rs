//! Error types for code-digest

use thiserror::Error;

/// Main error type for code-digest operations
#[derive(Error, Debug)]
pub enum CodeDigestError {
    /// File system related errors
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Failed to read file: {0}")]
    ReadError(String),

    #[error("Failed to write file: {0}")]
    WriteError(String),

    /// Configuration errors
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),

    #[error("Configuration file error: {0}")]
    ConfigError(String),

    /// Processing errors
    #[error("Token counting error: {0}")]
    TokenCountError(String),

    #[error("Markdown generation error: {0}")]
    MarkdownGenerationError(String),

    #[error("File prioritization error: {0}")]
    PrioritizationError(String),

    /// External tool errors
    #[error("{tool} not found. {install_instructions}")]
    LlmToolNotFound { tool: String, install_instructions: String },

    #[error("Subprocess error: {0}")]
    SubprocessError(String),

    /// Resource limits
    #[error("File too large: {0} (max: {1} bytes)")]
    FileTooLarge(String, usize),

    #[error("Token limit exceeded: {current} tokens (max: {max})")]
    TokenLimitExceeded { current: usize, max: usize },

    /// Pattern matching errors
    #[error("Invalid glob pattern: {0}")]
    InvalidGlobPattern(String),

    /// Remote repository errors
    #[error("Remote fetch error: {0}")]
    RemoteFetchError(String),

    /// Clipboard errors
    #[error("Clipboard error: {0}")]
    ClipboardError(String),

    /// General I/O errors
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// UTF-8 conversion errors
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// Result type alias for code-digest operations
pub type Result<T> = std::result::Result<T, CodeDigestError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CodeDigestError::InvalidPath("/invalid/path".to_string());
        assert_eq!(err.to_string(), "Invalid path: /invalid/path");

        let err = CodeDigestError::TokenLimitExceeded { current: 200000, max: 150000 };
        assert_eq!(err.to_string(), "Token limit exceeded: 200000 tokens (max: 150000)");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: CodeDigestError = io_err.into();
        assert!(matches!(err, CodeDigestError::IoError(_)));
    }
}
