//! Consolidated test runner for all integration tests
//!
//! This file includes all test modules to create a single compilation unit,
//! significantly reducing compilation time when running tests.

// Helper modules must be declared first
#[path = "modules/semantic_test_helpers.rs"]
#[allow(dead_code)]
#[allow(clippy::duplicate_mod)]
mod semantic_test_helpers;

// Core functionality tests
#[path = "modules/binary_filtering_integration_test.rs"]
mod binary_filtering_integration_test;
#[path = "modules/binary_name_test.rs"]
mod binary_name_test;
#[path = "modules/cache_integration_test.rs"]
mod cache_integration_test;
#[path = "modules/config_precedence_test.rs"]
mod config_precedence_test;
#[path = "modules/config_rename_test.rs"]
mod config_rename_test;
#[path = "modules/content_hash_internal_test.rs"]
mod content_hash_internal_test;
#[path = "modules/content_hash_test.rs"]
mod content_hash_test;
#[path = "modules/integration_test.rs"]
mod integration_test;
#[path = "modules/module_rename_test.rs"]
mod module_rename_test;

// CLI tests
#[path = "modules/cli_combinations_test.rs"]
mod cli_combinations_test;
#[path = "modules/cli_flexibility_test.rs"]
mod cli_flexibility_test;
#[path = "modules/cli_repo_paths_bug_test.rs"]
mod cli_repo_paths_bug_test;
#[path = "modules/cli_test.rs"]
mod cli_test;
#[path = "modules/cli_uncovered_scenarios_test.rs"]
mod cli_uncovered_scenarios_test;
#[path = "modules/diff_cli_test.rs"]
mod diff_cli_test;
#[path = "modules/diff_functionality_missing_test.rs"]
mod diff_functionality_missing_test;
#[path = "modules/diff_security_vulnerabilities_test.rs"]
mod diff_security_vulnerabilities_test;
#[path = "modules/git_utilities_test.rs"]
mod git_utilities_test;
#[path = "modules/git_utilities_vulnerability_test.rs"]
mod git_utilities_vulnerability_test;
#[path = "modules/logging_test.rs"]
mod logging_test;
#[path = "modules/search_acceptance_test.rs"]
mod search_acceptance_test;
#[path = "modules/search_command_test.rs"]
mod search_command_test;
#[path = "modules/search_gitignore_test.rs"]
mod search_gitignore_test;
#[path = "modules/search_integration_test.rs"]
mod search_integration_test;
#[path = "modules/search_semantic_test.rs"]
mod search_semantic_test;
#[path = "modules/search_test.rs"]
mod search_test;

// Pattern and ignore tests
#[path = "modules/glob_pattern_test.rs"]
mod glob_pattern_test;
#[path = "modules/ignore_patterns_test.rs"]
mod ignore_patterns_test;

// Output format tests
#[path = "modules/formatters_test.rs"]
mod formatters_test;
#[path = "modules/output_format_test.rs"]
mod output_format_test;

// Semantic analysis tests
#[path = "integration/cli_include_callers_simple_test.rs"]
mod cli_include_callers_simple_test;
#[path = "integration/cli_include_callers_test.rs"]
mod cli_include_callers_test;
#[path = "modules/cycle_detection_integration.rs"]
mod cycle_detection_integration;
#[path = "modules/cycle_detection_test.rs"]
mod cycle_detection_test;
#[path = "modules/cycle_detection_warning_test.rs"]
mod cycle_detection_warning_test;
#[path = "modules/edge_typing_test.rs"]
mod edge_typing_test;
#[path = "integration/include_callers_real_repos_test.rs"]
mod include_callers_real_repos_test;
#[path = "modules/integration_trace_imports_test.rs"]
mod integration_trace_imports_test;
#[path = "modules/parallel_semantic_test.rs"]
mod parallel_semantic_test;
#[path = "modules/parallel_workflow_test.rs"]
mod parallel_workflow_test;
#[path = "modules/semantic_analysis_test.rs"]
mod semantic_analysis_test;
#[path = "modules/semantic_comprehensive_test.rs"]
mod semantic_comprehensive_test;
#[path = "modules/semantic_edge_cases_test.rs"]
mod semantic_edge_cases_test;
#[path = "modules/semantic_error_cases_test.rs"]
mod semantic_error_cases_test;
#[path = "modules/semantic_include_callers_test.rs"]
mod semantic_include_callers_test;
#[path = "modules/semantic_include_types_integration_test.rs"]
mod semantic_include_types_integration_test;
#[path = "modules/semantic_include_types_simple_test.rs"]
mod semantic_include_types_simple_test;
#[path = "modules/semantic_include_types_test.rs"]
mod semantic_include_types_test;
#[path = "modules/semantic_markdown_test.rs"]
mod semantic_markdown_test;
#[path = "modules/semantic_output_test.rs"]
mod semantic_output_test;
#[path = "modules/semantic_refactor_integration.rs"]
mod semantic_refactor_integration;
#[path = "modules/semantic_reliability_test.rs"]
mod semantic_reliability_test;
#[path = "modules/semantic_trace_imports_test.rs"]
mod semantic_trace_imports_test;

// Python semantic tests
#[path = "modules/python_semantic_edge_cases_test.rs"]
mod python_semantic_edge_cases_test;
#[path = "modules/python_semantic_error_cases_test.rs"]
mod python_semantic_error_cases_test;
#[path = "modules/python_semantic_output_test.rs"]
mod python_semantic_output_test;

// End-to-end tests
#[path = "modules/e2e_test.rs"]
mod e2e_test;

// Token and prompt tests
#[path = "modules/prompt_token_reservation_test.rs"]
mod prompt_token_reservation_test;
#[path = "modules/token_limits_integration_test.rs"]
mod token_limits_integration_test;

// Remote repository tests
#[path = "modules/remote_parsing_test.rs"]
mod remote_parsing_test;

// Security tests
#[path = "modules/security_vulnerability_test.rs"]
mod security_vulnerability_test;

// Acceptance tests
#[path = "modules/acceptance/mod.rs"]
mod acceptance;

// Edge case tests
#[path = "modules/edge_cases/mod.rs"]
mod edge_cases;
