//! Benchmarks for type resolution with circuit breakers

use context_creator::core::semantic::analyzer::TypeReference;
use context_creator::core::semantic::type_resolver::{ResolutionLimits, TypeResolver};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use std::time::Duration;

fn create_type_reference(name: &str, depth: usize) -> TypeReference {
    TypeReference {
        name: format!("Type{name}_{depth}"),
        module: Some(format!("module::level{depth}")),
        line: 1,
        definition_path: Some(PathBuf::from(format!("/path/to/type{name}_{depth}.rs"))),
        is_external: false,
        external_package: None,
    }
}

fn benchmark_resolution_with_circuit_breakers(c: &mut Criterion) {
    c.bench_function("type_resolution_with_breakers", |b| {
        let limits = ResolutionLimits {
            max_depth: 20,
            max_visited_types: 100,
            max_resolution_time: Duration::from_secs(10),
        };
        let mut resolver = TypeResolver::with_limits(limits);

        b.iter(|| {
            for i in 0..10 {
                let type_ref = create_type_reference("BenchType", i);
                let _result = resolver.resolve_with_limits(black_box(&type_ref), black_box(i));
            }
            resolver.clear_cache();
        });
    });
}

fn benchmark_resolution_without_circuit_breakers(c: &mut Criterion) {
    c.bench_function("type_resolution_no_limits", |b| {
        // Simulate resolution without circuit breakers by setting very high limits
        let limits = ResolutionLimits {
            max_depth: 1000,
            max_visited_types: 10000,
            max_resolution_time: Duration::from_secs(3600),
        };
        let mut resolver = TypeResolver::with_limits(limits);

        b.iter(|| {
            for i in 0..10 {
                let type_ref = create_type_reference("BenchType", i);
                let _result = resolver.resolve_with_limits(black_box(&type_ref), black_box(i));
            }
            resolver.clear_cache();
        });
    });
}

fn benchmark_deep_type_hierarchy(c: &mut Criterion) {
    c.bench_function("deep_type_hierarchy", |b| {
        let limits = ResolutionLimits {
            max_depth: 50,
            max_visited_types: 200,
            max_resolution_time: Duration::from_secs(10),
        };
        let mut resolver = TypeResolver::with_limits(limits);

        b.iter(|| {
            // Simulate resolving a deep type hierarchy
            for depth in 0..30 {
                let type_ref = create_type_reference("DeepType", depth);
                let _result = resolver.resolve_with_limits(black_box(&type_ref), black_box(depth));
            }
        });
    });
}

fn benchmark_cache_performance(c: &mut Criterion) {
    c.bench_function("type_resolution_cache_hits", |b| {
        let limits = ResolutionLimits::default();
        let mut resolver = TypeResolver::with_limits(limits);

        // Pre-populate cache
        for i in 0..20 {
            let type_ref = create_type_reference("CachedType", i);
            let _ = resolver.resolve_with_limits(&type_ref, 0);
        }

        b.iter(|| {
            // Resolve the same types again - should hit cache
            for i in 0..20 {
                let type_ref = create_type_reference("CachedType", i);
                let _result = resolver.resolve_with_limits(black_box(&type_ref), black_box(0));
            }
        });
    });
}

fn benchmark_circuit_breaker_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("circuit_breaker_overhead");

    // Benchmark with minimal checks
    group.bench_function("minimal_checks", |b| {
        let limits = ResolutionLimits {
            max_depth: 1000,
            max_visited_types: 10000,
            max_resolution_time: Duration::from_secs(3600),
        };
        let resolver = TypeResolver::with_limits(limits);

        b.iter(|| resolver.is_circuit_breaker_triggered(black_box(5), black_box(10)));
    });

    // Benchmark with typical checks
    group.bench_function("typical_checks", |b| {
        let limits = ResolutionLimits::default();
        let resolver = TypeResolver::with_limits(limits);

        b.iter(|| resolver.is_circuit_breaker_triggered(black_box(5), black_box(10)));
    });

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_usage_deep_hierarchy", |b| {
        b.iter(|| {
            let limits = ResolutionLimits {
                max_depth: 100,
                max_visited_types: 500,
                max_resolution_time: Duration::from_secs(30),
            };
            let mut resolver = TypeResolver::with_limits(limits);

            // Create a large number of types to test memory usage
            for i in 0..100 {
                for j in 0..5 {
                    let type_ref = TypeReference {
                        name: format!("Type_{i}_{j}"),
                        module: Some(format!("module{i}")),
                        line: 1,
                        definition_path: Some(PathBuf::from(format!("/path/{i}/{j}.rs"))),
                        is_external: false,
                        external_package: None,
                    };
                    let _ = resolver.resolve_with_limits(&type_ref, i % 20);
                }
            }

            // Return cache stats to prevent optimization
            black_box(resolver.cache_stats())
        });
    });
}

criterion_group!(
    benches,
    benchmark_resolution_with_circuit_breakers,
    benchmark_resolution_without_circuit_breakers,
    benchmark_deep_type_hierarchy,
    benchmark_cache_performance,
    benchmark_circuit_breaker_overhead,
    benchmark_memory_usage
);
criterion_main!(benches);
