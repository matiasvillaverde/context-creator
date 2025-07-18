//! Type resolution with circuit breakers to prevent infinite loops and resource exhaustion
//!
//! This module provides a type resolver that includes multiple safety mechanisms:
//! - Depth limiting to prevent stack overflow
//! - Visited type tracking to detect circular references
//! - Resolution caching to improve performance
//! - Time-based circuit breakers for long-running resolutions

use crate::core::semantic::analyzer::TypeReference;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for type resolution limits
#[derive(Debug, Clone)]
pub struct ResolutionLimits {
    /// Maximum depth for type resolution
    pub max_depth: usize,
    /// Maximum number of types to visit before stopping
    pub max_visited_types: usize,
    /// Maximum time allowed for a single resolution
    pub max_resolution_time: Duration,
}

impl Default for ResolutionLimits {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_visited_types: 100,
            max_resolution_time: Duration::from_secs(5),
        }
    }
}

/// Type resolver with circuit breaker capabilities
pub struct TypeResolver {
    /// Maximum depth for type resolution
    max_depth: usize,
    /// Set of visited types in current resolution chain
    visited_types: HashSet<String>,
    /// Cache of previously resolved types
    resolution_cache: HashMap<String, Option<PathBuf>>,
    /// Start time of current resolution
    resolution_start: Option<Instant>,
    /// Resolution limits configuration
    limits: ResolutionLimits,
}

impl TypeResolver {
    /// Create a new type resolver with default limits
    pub fn new() -> Self {
        Self::with_limits(ResolutionLimits::default())
    }

    /// Create a new type resolver with custom limits
    pub fn with_limits(limits: ResolutionLimits) -> Self {
        Self {
            max_depth: limits.max_depth,
            visited_types: HashSet::new(),
            resolution_cache: HashMap::new(),
            resolution_start: None,
            limits,
        }
    }

    /// Resolve a type reference with circuit breaker protections
    pub fn resolve_with_limits(
        &mut self,
        type_ref: &TypeReference,
        current_depth: usize,
    ) -> Result<Option<PathBuf>, String> {
        // Start timing if this is the first call
        if self.resolution_start.is_none() {
            self.resolution_start = Some(Instant::now());
        }

        // Check circuit breakers
        if self.is_circuit_breaker_triggered(current_depth, self.visited_types.len()) {
            return Err("Circuit breaker triggered: resolution limits exceeded".to_string());
        }

        // Check for circular reference first (before cache)
        let cache_key = self.create_cache_key(type_ref);
        if self.visited_types.contains(&cache_key) {
            return Err(format!(
                "Circular type reference detected: {}",
                type_ref.name
            ));
        }

        // Check cache
        if let Some(cached_result) = self.resolution_cache.get(&cache_key) {
            return Ok(cached_result.clone());
        }

        // Mark type as visited
        self.visited_types.insert(cache_key.clone());

        // Simulate type resolution (in real implementation, this would call actual resolution logic)
        let result = if type_ref.is_external {
            // External types don't need file resolution
            None
        } else {
            // For internal types, use the definition path if available
            type_ref.definition_path.clone()
        };

        // Cache the result
        self.resolution_cache.insert(cache_key, result.clone());

        Ok(result)
    }

    /// Check if any circuit breaker condition is triggered
    pub fn is_circuit_breaker_triggered(&self, depth: usize, visited_count: usize) -> bool {
        // Check depth limit
        if depth >= self.max_depth {
            return true;
        }

        // Check visited types limit
        if visited_count >= self.limits.max_visited_types {
            return true;
        }

        // Check time limit
        if let Some(start_time) = self.resolution_start {
            if start_time.elapsed() > self.limits.max_resolution_time {
                return true;
            }
        }

        false
    }

    /// Create a cache key for a type reference
    fn create_cache_key(&self, type_ref: &TypeReference) -> String {
        if let Some(module) = &type_ref.module {
            format!("{}::{}", module, type_ref.name)
        } else {
            type_ref.name.clone()
        }
    }

    /// Clear the resolution cache
    pub fn clear_cache(&mut self) {
        self.resolution_cache.clear();
    }

    /// Clear the current resolution state (visited types and timer)
    pub fn clear_resolution_state(&mut self) {
        self.visited_types.clear();
        self.resolution_start = None;
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let total_entries = self.resolution_cache.len();
        let resolved_entries = self
            .resolution_cache
            .values()
            .filter(|v| v.is_some())
            .count();
        (total_entries, resolved_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_type_ref(name: &str, module: Option<&str>) -> TypeReference {
        TypeReference {
            name: name.to_string(),
            module: module.map(|s| s.to_string()),
            line: 1,
            definition_path: None,
            is_external: false,
            external_package: None,
        }
    }

    #[test]
    fn test_depth_limit_enforcement() {
        let limits = ResolutionLimits {
            max_depth: 3,
            ..Default::default()
        };
        let mut resolver = TypeResolver::with_limits(limits);

        // Should succeed at depth 0, 1, 2
        for depth in 0..3 {
            let type_ref = create_test_type_ref(&format!("Type{depth}"), None);
            let result = resolver.resolve_with_limits(&type_ref, depth);
            assert!(result.is_ok(), "Should succeed at depth {depth}");
        }

        // Should fail at depth 3
        let type_ref = create_test_type_ref("Type3", None);
        let result = resolver.resolve_with_limits(&type_ref, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circuit breaker triggered"));
    }

    #[test]
    fn test_circular_type_reference() {
        let mut resolver = TypeResolver::new();

        // Create type A
        let type_a = create_test_type_ref("A", Some("module"));

        // First resolution should succeed
        let result1 = resolver.resolve_with_limits(&type_a, 0);
        assert!(result1.is_ok());

        // Don't clear state - simulate being in the same resolution chain
        // Simulate visiting A again in the same resolution chain
        // (In real usage, this would happen through nested resolution)
        let result2 = resolver.resolve_with_limits(&type_a, 1);
        assert!(result2.is_err());
        assert!(result2
            .unwrap_err()
            .contains("Circular type reference detected"));
    }

    #[test]
    fn test_deeply_nested_types() {
        let limits = ResolutionLimits {
            max_depth: 15,
            max_visited_types: 20,
            ..Default::default()
        };
        let mut resolver = TypeResolver::with_limits(limits);

        // Create a chain of 12 different types in a single resolution chain
        for i in 0..12 {
            let type_ref = create_test_type_ref(&format!("NestedType{i}"), Some("deep"));
            let result = resolver.resolve_with_limits(&type_ref, i);
            assert!(result.is_ok(), "Should handle nested type at level {i}");
        }

        // Verify we visited all types (they should still be in visited_types)
        assert!(
            resolver.visited_types.len() >= 12,
            "Expected at least 12 visited types, got {}",
            resolver.visited_types.len()
        );
    }

    #[test]
    fn test_resolution_timeout() {
        let limits = ResolutionLimits {
            max_resolution_time: Duration::from_millis(100),
            ..Default::default()
        };
        let mut resolver = TypeResolver::with_limits(limits);

        // First resolution starts the timer
        let type_ref = create_test_type_ref("SlowType", None);
        let _result = resolver.resolve_with_limits(&type_ref, 0);

        // Simulate time passing by sleeping
        std::thread::sleep(Duration::from_millis(150));

        // Next resolution should trigger timeout
        let type_ref2 = create_test_type_ref("AnotherType", None);
        let result = resolver.resolve_with_limits(&type_ref2, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circuit breaker triggered"));
    }

    #[test]
    fn test_resolution_with_cache() {
        let mut resolver = TypeResolver::new();

        // Create a type with a definition path
        let mut type_ref = create_test_type_ref("CachedType", Some("module"));
        type_ref.definition_path = Some(PathBuf::from("/path/to/cached_type.rs"));

        // First resolution should work
        let result1 = resolver.resolve_with_limits(&type_ref, 0);
        assert!(result1.is_ok());
        assert_eq!(
            result1.unwrap(),
            Some(PathBuf::from("/path/to/cached_type.rs"))
        );

        // Clear resolution state to simulate a new resolution chain
        resolver.clear_resolution_state();

        // Second resolution should use cache and not be considered a circular reference
        let result2 = resolver.resolve_with_limits(&type_ref, 0);
        assert!(result2.is_ok());
        assert_eq!(
            result2.unwrap(),
            Some(PathBuf::from("/path/to/cached_type.rs"))
        );

        // Verify cache statistics
        let (total, resolved) = resolver.cache_stats();
        assert_eq!(total, 1);
        assert_eq!(resolved, 1);
    }

    #[test]
    fn test_visited_types_limit() {
        let limits = ResolutionLimits {
            max_visited_types: 5,
            ..Default::default()
        };
        let mut resolver = TypeResolver::with_limits(limits);

        // Visit 5 different types in the same resolution chain
        for i in 0..5 {
            let type_ref = create_test_type_ref(&format!("Type{i}"), Some("module"));
            let result = resolver.resolve_with_limits(&type_ref, i);
            assert!(result.is_ok());
            // Don't clear visited types to simulate all in one resolution chain
        }

        // 6th type should trigger the limit
        let type_ref = create_test_type_ref("Type5", Some("module"));
        let result = resolver.resolve_with_limits(&type_ref, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circuit breaker triggered"));
    }

    #[test]
    fn test_external_types() {
        let mut resolver = TypeResolver::new();

        // Create an external type
        let mut type_ref = create_test_type_ref("HashMap", Some("std::collections"));
        type_ref.is_external = true;
        type_ref.external_package = Some("std v1.0.0".to_string());

        // External types should resolve to None (no file path)
        let result = resolver.resolve_with_limits(&type_ref, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_cache_key_generation() {
        let resolver = TypeResolver::new();

        // Type with module
        let type1 = create_test_type_ref("MyType", Some("my::module"));
        assert_eq!(resolver.create_cache_key(&type1), "my::module::MyType");

        // Type without module
        let type2 = create_test_type_ref("SimpleType", None);
        assert_eq!(resolver.create_cache_key(&type2), "SimpleType");
    }
}
