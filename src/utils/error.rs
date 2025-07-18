//! Error types for context-creator

use thiserror::Error;

/// Main error type for context-creator operations
#[derive(Error, Debug)]
pub enum ContextCreatorError {
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

    #[error("Context generation error: {0}")]
    ContextGenerationError(String),

    #[error("File prioritization error: {0}")]
    PrioritizationError(String),

    /// External tool errors
    #[error("{tool} not found. {install_instructions}")]
    LlmToolNotFound {
        tool: String,
        install_instructions: String,
    },

    #[error("Subprocess error: {0}")]
    SubprocessError(String),

    /// Resource limits
    #[error("File too large: {0} (max: {1} bytes)")]
    FileTooLarge(String, usize),

    #[error("Token limit exceeded: {current} tokens (max: {max})")]
    TokenLimitExceeded { current: usize, max: usize },

    /// Parsing errors
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Pattern matching errors
    #[error("Invalid glob pattern: {0}")]
    InvalidGlobPattern(String),

    /// Mutex errors
    #[error("Mutex was poisoned, indicating a previous panic")]
    MutexPoisoned,

    /// Security errors
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Remote repository errors
    #[error("Remote fetch error: {0}")]
    RemoteFetchError(String),

    /// Clipboard errors
    #[error("Clipboard error: {0}")]
    ClipboardError(String),

    /// Parallel processing errors
    #[error("File processing error for {path}: {error}")]
    FileProcessingError { path: String, error: String },

    #[error("Token counting error for {path}: {error}")]
    TokenCountingError { path: String, error: String },

    #[error("Parallel processing errors: {error_count} files failed")]
    ParallelProcessingErrors { error_count: usize },

    /// General I/O errors
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// UTF-8 conversion errors
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// Result type alias for context-creator operations
pub type Result<T> = std::result::Result<T, ContextCreatorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ContextCreatorError::InvalidPath("/invalid/path".to_string());
        assert_eq!(err.to_string(), "Invalid path: /invalid/path");

        let err = ContextCreatorError::TokenLimitExceeded {
            current: 200_000,
            max: 150_000,
        };
        assert_eq!(
            err.to_string(),
            "Token limit exceeded: 200000 tokens (max: 150000)"
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ContextCreatorError = io_err.into();
        assert!(matches!(err, ContextCreatorError::IoError(_)));
    }
}
