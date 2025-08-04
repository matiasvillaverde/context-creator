//! Telemetry command implementation for enriching source code with OpenTelemetry data

use crate::cli::{Commands, Config};
use crate::core::semantic::parallel_analyzer::{AnalysisOptions, ParallelAnalyzer};
use crate::core::telemetry::{
    JsonParser, OtlpParser, ProtobufParser, TelemetryCorrelator, TelemetryEnricher,
};
use crate::core::{
    cache::FileCache,
    walker::{walk_directory, WalkOptions},
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Parse a time range string in format "start/end" where times are RFC3339
fn parse_time_range(time_range: &str) -> Result<(u64, u64)> {
    let parts: Vec<&str> = time_range.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid time range format. Expected 'start/end' in RFC3339 format");
    }

    let start_time = parse_rfc3339_to_nanos(parts[0])
        .with_context(|| format!("Failed to parse start time: {}", parts[0]))?;
    let end_time = parse_rfc3339_to_nanos(parts[1])
        .with_context(|| format!("Failed to parse end time: {}", parts[1]))?;

    if start_time >= end_time {
        anyhow::bail!("Start time must be before end time");
    }

    Ok((start_time, end_time))
}

/// Parse an RFC3339 timestamp to nanoseconds since Unix epoch
fn parse_rfc3339_to_nanos(timestamp: &str) -> Result<u64> {
    // Parse RFC3339 timestamp (e.g., "2024-01-01T00:00:00Z")
    // For now, we'll use a simple approach. In a real implementation,
    // you might want to use chrono or time crate for proper parsing

    // Basic validation
    if !timestamp.ends_with('Z') {
        anyhow::bail!("Only UTC timestamps (ending with 'Z') are supported");
    }

    // Remove the 'Z' and split by 'T'
    let timestamp_without_z = &timestamp[..timestamp.len() - 1];
    let parts: Vec<&str> = timestamp_without_z.split('T').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid timestamp format");
    }

    let date_part = parts[0];
    let time_part = parts[1];

    // Parse date part (YYYY-MM-DD)
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        anyhow::bail!("Invalid date format");
    }
    let year: i32 = date_parts[0].parse().context("Invalid year")?;
    let month: u32 = date_parts[1].parse().context("Invalid month")?;
    let day: u32 = date_parts[2].parse().context("Invalid day")?;

    // Parse time part (HH:MM:SS)
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() != 3 {
        anyhow::bail!("Invalid time format");
    }
    let hour: u32 = time_parts[0].parse().context("Invalid hour")?;
    let minute: u32 = time_parts[1].parse().context("Invalid minute")?;
    let second: u32 = time_parts[2].parse().context("Invalid second")?;

    // Basic validation
    if !(1..=12).contains(&month) {
        anyhow::bail!("Invalid month: {}", month);
    }
    if !(1..=31).contains(&day) {
        anyhow::bail!("Invalid day: {}", day);
    }
    if hour > 23 {
        anyhow::bail!("Invalid hour: {}", hour);
    }
    if minute > 59 {
        anyhow::bail!("Invalid minute: {}", minute);
    }
    if second > 59 {
        anyhow::bail!("Invalid second: {}", second);
    }

    // Calculate Unix timestamp (simplified calculation for 2024 onwards)
    // This is a simplified implementation - in production, use a proper date/time library
    let days_since_epoch = if year >= 2024 {
        // Approximate calculation for years >= 2024
        let _days_in_year = if year % 4 == 0 { 366 } else { 365 };
        let years_since_2024 = (year - 2024) as u64;
        let base_days = 19723; // Approximate days from 1970-01-01 to 2024-01-01
        let days_from_years = years_since_2024 * 365 + years_since_2024 / 4; // Rough leap year adjustment

        // Days from start of year
        let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut days_from_months = 0;
        for m in 1..month {
            days_from_months += days_in_months[(m - 1) as usize];
        }
        // Add leap day if needed
        if month > 2 && year % 4 == 0 {
            days_from_months += 1;
        }

        base_days + days_from_years + days_from_months as u64 + (day - 1) as u64
    } else {
        anyhow::bail!(
            "Only dates from 2024 onwards are supported in this simplified implementation"
        );
    };

    let seconds_since_epoch = days_since_epoch * 24 * 60 * 60
        + hour as u64 * 60 * 60
        + minute as u64 * 60
        + second as u64;
    let nanos_since_epoch = seconds_since_epoch * 1_000_000_000;

    Ok(nanos_since_epoch)
}

/// Run the telemetry command to enrich source code with OpenTelemetry data
pub fn run_telemetry(config: Config) -> Result<()> {
    // Extract telemetry command arguments
    let (telemetry_file, time_range, service, paths) = match &config.command {
        Some(Commands::Telemetry {
            telemetry_file,
            time_range,
            service,
            paths,
        }) => (
            telemetry_file.clone(),
            time_range.clone(),
            service.clone(),
            paths.clone(),
        ),
        _ => anyhow::bail!("Invalid command for telemetry execution"),
    };

    // Validate telemetry file exists
    if !telemetry_file.exists() {
        anyhow::bail!(
            "Telemetry file does not exist: {}",
            telemetry_file.display()
        );
    }

    // Determine parser based on file extension
    let parser: Box<dyn OtlpParser> = if telemetry_file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "json")
        .unwrap_or(false)
    {
        Box::new(JsonParser::new())
    } else {
        Box::new(ProtobufParser::new())
    };

    // Parse telemetry data
    if config.progress && !config.quiet {
        info!("Parsing telemetry file: {}", telemetry_file.display());
    } else {
        debug!("Parsing telemetry file: {}", telemetry_file.display());
    }
    let parsed = parser.parse_file(&telemetry_file).with_context(|| {
        format!(
            "Failed to parse telemetry file: {} (check file format and permissions)",
            telemetry_file.display()
        )
    })?;

    // Store total count before filtering
    let total_spans = parsed.spans.len();

    // Early validation: warn if no spans found
    if total_spans == 0 {
        warn!(
            "No telemetry spans found in file: {}",
            telemetry_file.display()
        );
    }

    // Apply service filter if provided
    let filtered_spans = if let Some(service_filter) = service {
        parsed
            .spans
            .into_iter()
            .filter(|span| span.service_name.as_ref() == Some(&service_filter))
            .collect()
    } else {
        parsed.spans
    };

    // Apply time range filter if provided
    let time_filtered_spans = if let Some(time_range_str) = time_range {
        let (start_nanos, end_nanos) = parse_time_range(&time_range_str)
            .with_context(|| format!("Failed to parse time range: {time_range_str}"))?;

        if config.progress && !config.quiet {
            info!("Filtering spans by time range: {}", time_range_str);
        } else {
            debug!("Filtering spans by time range: {}", time_range_str);
        }

        filtered_spans
            .into_iter()
            .filter(|span| {
                span.start_time_nanos >= start_nanos && span.start_time_nanos <= end_nanos
            })
            .collect()
    } else {
        filtered_spans
    };

    // Determine paths to analyze
    let analysis_paths = paths.unwrap_or_else(|| {
        vec![std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))]
    });

    // Analyze project to get function information
    let mut all_files = Vec::new();
    let cache = Arc::new(FileCache::new());

    for path in &analysis_paths {
        let walk_options = WalkOptions::from_config(&config)?;
        let files = walk_directory(path, walk_options)?;
        all_files.extend(files);
    }

    // Perform semantic analysis to get function definitions
    let base_path = analysis_paths
        .first()
        .with_context(|| "No analysis paths provided")?
        .clone();
    let analyzer = ParallelAnalyzer::new(&cache);
    let analysis_options = AnalysisOptions {
        semantic_depth: 0, // We only need function definitions
        trace_imports: false,
        include_types: false,
        include_functions: true,
    };

    // Get file paths for analysis
    let file_paths: Vec<PathBuf> = all_files.iter().map(|f| f.path.clone()).collect();
    let valid_files = file_paths.iter().cloned().collect();

    // Analyze files to get function definitions
    if config.progress && !config.quiet {
        info!("Analyzing {} source files...", file_paths.len());
    } else {
        debug!("Analyzing {} source files...", file_paths.len());
    }
    let analysis_results = analyzer
        .analyze_files(&file_paths, &base_path, &analysis_options, &valid_files)
        .with_context(|| {
            format!(
                "Failed to analyze {} source files in path: {}",
                file_paths.len(),
                base_path.display()
            )
        })?;

    // Create correlator and correlate spans
    if config.progress && !config.quiet {
        info!("Correlating telemetry data with source code...");
    } else {
        debug!("Correlating telemetry data with source code...");
    }
    let correlator = TelemetryCorrelator::new(analysis_results, file_paths, base_path);
    let correlation_result = correlator.correlate_spans(time_filtered_spans);

    // Log correlation details for debugging
    info!(
        total_spans = total_spans,
        correlated_spans = correlation_result.correlated_count,
        uncorrelated_spans = correlation_result.uncorrelated_spans.len(),
        functions_with_metrics = correlation_result.function_metrics.len(),
        "Telemetry correlation completed"
    );

    // Print correlation summary for user
    println!("Telemetry Correlation Summary:");
    println!("  Total spans: {total_spans}");
    println!(
        "  Correlated spans: {}",
        correlation_result.correlated_count
    );
    println!(
        "  Uncorrelated spans: {}",
        correlation_result.uncorrelated_spans.len()
    );
    println!(
        "  Functions with metrics: {}",
        correlation_result.function_metrics.len()
    );

    // Generate enriched output
    let enricher = TelemetryEnricher::new(correlation_result);
    let summary = enricher.generate_summary();

    // Print the summary
    println!("\n{summary}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc3339_to_nanos() {
        // Test basic 2024 timestamp
        let timestamp = "2024-01-01T00:00:00Z";
        let result = parse_rfc3339_to_nanos(timestamp).unwrap();

        // 2024-01-01T00:00:00Z should be valid
        assert!(result > 0);

        // Test with specific time
        let timestamp2 = "2024-01-01T00:00:01Z";
        let result2 = parse_rfc3339_to_nanos(timestamp2).unwrap();

        // Should be exactly 1 second (1 billion nanoseconds) later
        assert_eq!(result2 - result, 1_000_000_000);
    }

    #[test]
    fn test_parse_time_range() {
        let time_range = "2024-01-01T00:00:00Z/2024-01-01T00:00:01Z";
        let (start, end) = parse_time_range(time_range).unwrap();

        // End should be 1 second after start
        assert_eq!(end - start, 1_000_000_000);
        assert!(start < end);
    }

    #[test]
    fn test_parse_time_range_invalid_format() {
        let time_range = "invalid";
        let result = parse_time_range(time_range);
        assert!(result.is_err());

        let time_range2 = "2024-01-01T00:00:00Z";
        let result2 = parse_time_range(time_range2);
        assert!(result2.is_err());
    }

    #[test]
    fn test_parse_rfc3339_validation() {
        // Invalid formats should fail
        assert!(parse_rfc3339_to_nanos("invalid").is_err());
        assert!(parse_rfc3339_to_nanos("2024-01-01").is_err());
        assert!(parse_rfc3339_to_nanos("2024-01-01T25:00:00Z").is_err()); // Invalid hour
        assert!(parse_rfc3339_to_nanos("2024-13-01T00:00:00Z").is_err()); // Invalid month
        assert!(parse_rfc3339_to_nanos("2023-01-01T00:00:00Z").is_err()); // Unsupported year
    }

    #[test]
    fn test_parse_time_range_edge_cases() {
        // Test with same start and end time (should fail)
        let same_time = "2024-01-01T00:00:00Z/2024-01-01T00:00:00Z";
        assert!(parse_time_range(same_time).is_err());

        // Test with reversed times (should fail)
        let reversed = "2024-01-01T00:00:01Z/2024-01-01T00:00:00Z";
        assert!(parse_time_range(reversed).is_err());

        // Test with valid range
        let valid = "2024-01-01T00:00:00Z/2024-01-01T00:00:10Z";
        let (start, end) = parse_time_range(valid).unwrap();
        assert_eq!(end - start, 10_000_000_000); // 10 seconds in nanoseconds
    }

    #[test]
    fn test_rfc3339_parsing_accuracy() {
        // Test specific date calculations
        let jan_1_2024 = parse_rfc3339_to_nanos("2024-01-01T00:00:00Z").unwrap();
        let jan_2_2024 = parse_rfc3339_to_nanos("2024-01-02T00:00:00Z").unwrap();

        // Should be exactly 24 hours apart
        let day_in_nanos = 24 * 60 * 60 * 1_000_000_000;
        assert_eq!(jan_2_2024 - jan_1_2024, day_in_nanos);

        // Test time component parsing
        let midnight = parse_rfc3339_to_nanos("2024-01-01T00:00:00Z").unwrap();
        let one_am = parse_rfc3339_to_nanos("2024-01-01T01:00:00Z").unwrap();
        let one_minute = parse_rfc3339_to_nanos("2024-01-01T00:01:00Z").unwrap();
        let one_second = parse_rfc3339_to_nanos("2024-01-01T00:00:01Z").unwrap();

        assert_eq!(one_am - midnight, 3600 * 1_000_000_000); // 1 hour
        assert_eq!(one_minute - midnight, 60 * 1_000_000_000); // 1 minute
        assert_eq!(one_second - midnight, 1_000_000_000); // 1 second
    }

    #[test]
    fn test_time_range_format_variations() {
        // Test different valid time range formats
        let test_cases = [
            "2024-01-01T00:00:00Z/2024-01-01T00:00:01Z",
            "2024-12-31T23:59:59Z/2024-12-31T23:59:59Z", // This should actually fail
            "2024-02-29T12:30:45Z/2024-02-29T12:30:46Z", // Leap year
            "2024-06-15T09:30:00Z/2024-06-15T17:30:00Z", // 8 hour range
        ];

        let results: Vec<bool> = test_cases
            .iter()
            .map(|tc| parse_time_range(tc).is_ok())
            .collect();

        assert!(results[0]); // Valid 1-second range
        assert!(!results[1]); // Same time should fail
        assert!(results[2]); // Valid leap year date
        assert!(results[3]); // Valid 8-hour range
    }
}
