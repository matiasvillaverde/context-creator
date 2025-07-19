//! Logging configuration and initialization

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

use crate::{cli::LogFormat, Config};

/// Initialize the logging system based on configuration
pub fn init_logging(config: &Config) -> Result<()> {
    // Determine the log level based on flags
    let log_level = get_log_level(config.verbose, config.quiet, config.progress);

    // Create the base subscriber
    let builder = fmt::Subscriber::builder()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(std::io::stderr);

    // Check if RUST_LOG is set
    let env_filter = if std::env::var("RUST_LOG").is_ok() {
        // Use RUST_LOG if set
        EnvFilter::from_default_env()
    } else {
        // Otherwise use our level
        EnvFilter::new(format!("{log_level}"))
    };

    match config.log_format {
        LogFormat::Json => {
            // JSON format for structured logging
            builder.json().with_env_filter(env_filter).try_init().ok();
        }
        LogFormat::Plain => {
            // Human-readable format
            builder.with_env_filter(env_filter).try_init().ok();
        }
    }

    Ok(())
}

/// Get the appropriate log level based on verbose and quiet flags
pub fn get_log_level(verbose: u8, quiet: bool, progress: bool) -> Level {
    if quiet {
        Level::ERROR
    } else {
        match verbose {
            0 => {
                // If progress is enabled, show INFO level to display progress messages
                if progress {
                    Level::INFO
                } else {
                    Level::WARN // Default: show warnings and errors
                }
            }
            1 => Level::DEBUG, // -v: show debug logs
            _ => Level::TRACE, // -vv or more: show trace logs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_selection() {
        // Test quiet mode
        assert_eq!(get_log_level(0, true, false), Level::ERROR);
        assert_eq!(get_log_level(1, true, false), Level::ERROR);
        assert_eq!(get_log_level(2, true, false), Level::ERROR);

        // Test verbose levels
        assert_eq!(get_log_level(0, false, false), Level::WARN);
        assert_eq!(get_log_level(1, false, false), Level::DEBUG);
        assert_eq!(get_log_level(2, false, false), Level::TRACE);
        assert_eq!(get_log_level(3, false, false), Level::TRACE);

        // Test progress mode
        assert_eq!(get_log_level(0, false, true), Level::INFO);
        assert_eq!(get_log_level(0, true, true), Level::ERROR); // quiet overrides progress
    }
}
