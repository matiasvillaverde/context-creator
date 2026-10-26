//! Thread-safe parser pool for tree-sitter parsers
//! Manages parser lifecycle and prevents resource exhaustion

use crate::utils::error::ContextCreatorError;
use async_trait::async_trait;
use deadpool::managed::{self, Manager, Metrics, Pool, RecycleResult};
use std::collections::HashMap;
use tree_sitter::{Language, Parser};

/// Type alias for a pooled parser
pub type PooledParser = managed::Object<ParserManager>;

/// Type alias for a parser pool
pub type ParserPool = Pool<ParserManager>;

/// Manager for creating and recycling tree-sitter parsers
pub struct ParserManager {
    language: Language,
    language_name: &'static str,
}

impl ParserManager {
    /// Create a new parser manager for a specific language
    pub fn new(language: Language, language_name: &'static str) -> Self {
        Self {
            language,
            language_name,
        }
    }
}

#[async_trait]
impl Manager for ParserManager {
    type Type = Parser;
    type Error = ContextCreatorError;

    async fn create(&self) -> Result<Parser, Self::Error> {
        let mut parser = Parser::new();

        // Set the language
        parser.set_language(&self.language).map_err(|e| {
            ContextCreatorError::ParseError(format!(
                "Failed to set {} language: {}",
                self.language_name, e
            ))
        })?;

        Ok(parser)
    }

    async fn recycle(&self, parser: &mut Parser, _: &Metrics) -> RecycleResult<Self::Error> {
        // Reset the parser for reuse
        parser.reset();
        Ok(())
    }
}

/// Manages parser pools for multiple languages
pub struct ParserPoolManager {
    pools: HashMap<&'static str, ParserPool>,
}

impl ParserPoolManager {
    /// Create a new parser pool manager with pools for all supported languages
    pub fn new() -> Self {
        let mut pools = HashMap::new();

        // Create pools for each supported language
        // Each pool has a maximum of 16 parsers
        let pool_config = managed::PoolConfig {
            max_size: 16,
            ..Default::default()
        };

        for (name, path) in [
            ("rust", "file.rs"),
            ("python", "file.py"),
            ("javascript", "file.js"),
            ("typescript", "file.ts"),
            ("tsx", "file.tsx"),
            ("go", "file.go"),
            ("swift", "file.swift"),
            ("java", "file.java"),
            ("c", "file.c"),
            ("cpp", "file.cpp"),
            ("csharp", "file.cs"),
            ("kotlin", "file.kt"),
            ("ruby", "file.rb"),
            ("php", "file.php"),
            ("zig", "file.zig"),
            ("bash", "file.sh"),
            ("dart", "file.dart"),
            ("lua", "file.lua"),
            ("scala", "file.scala"),
            ("elixir", "file.ex"),
            ("r", "file.R"),
            ("julia", "file.jl"),
            ("haskell", "file.hs"),
            ("clojure", "file.clj"),
            ("powershell", "file.ps1"),
            ("perl", "file.pm"),
            ("sql", "file.sql"),
            ("hcl", "file.tf"),
            ("html", "file.html"),
            ("css", "file.css"),
        ] {
            if let Some(language) = super::grammar_for_file(std::path::Path::new(path)) {
                if let Ok(pool) = Pool::builder(ParserManager::new(language, name))
                    .config(pool_config)
                    .build()
                {
                    pools.insert(name, pool);
                }
            }
        }

        Self { pools }
    }

    /// Get a parser from the pool for the specified language
    pub async fn get_parser(&self, language: &str) -> Result<PooledParser, ContextCreatorError> {
        let pool = self.pools.get(language).ok_or_else(|| {
            ContextCreatorError::ParseError(format!("Unsupported language: {language}"))
        })?;

        pool.get().await.map_err(|e| {
            ContextCreatorError::ParseError(format!(
                "Failed to get {language} parser from pool: {e}"
            ))
        })
    }

    /// Get pool status for monitoring
    pub fn get_status(&self, language: &str) -> Option<deadpool::Status> {
        self.pools.get(language).map(|pool| pool.status())
    }
}

impl Default for ParserPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::semantic::parsing::ParserExt;

    #[tokio::test]
    async fn test_parser_creation() {
        let manager = ParserManager::new(tree_sitter_rust::LANGUAGE.into(), "rust");
        let mut parser = manager.create().await.unwrap();

        // The parser supports bounded parsing.
        assert!(parser.parse_bounded("fn main() {}", None).is_some());
    }

    #[tokio::test]
    async fn test_parser_recycling() {
        let manager = ParserManager::new(tree_sitter_python::LANGUAGE.into(), "python");
        let mut parser = manager.create().await.unwrap();

        // Recycling should succeed
        let result = manager.recycle(&mut parser, &Metrics::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pool_manager() {
        let pool_manager = ParserPoolManager::new();

        // Should successfully get parsers
        let rust_parser = pool_manager.get_parser("rust").await;
        assert!(rust_parser.is_ok());

        let python_parser = pool_manager.get_parser("python").await;
        assert!(python_parser.is_ok());

        // Should fail for unsupported language
        let unknown = pool_manager.get_parser("cobol").await;
        assert!(unknown.is_err());
    }
}
