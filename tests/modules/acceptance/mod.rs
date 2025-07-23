//! Comprehensive acceptance test suite for context-creator CLI
//!
//! This module contains acceptance tests that validate the complete CLI experience
//! by running the compiled binary against well-defined project structures and
//! asserting the correctness of generated Markdown output.

pub mod binary_filtering;
pub mod builders;
pub mod complex_combinations;
pub mod core_inclusion;
pub mod fixtures;
pub mod helpers;
pub mod semantic_callers;
pub mod semantic_imports;
pub mod semantic_types;

// Re-export common test utilities are handled by individual modules
