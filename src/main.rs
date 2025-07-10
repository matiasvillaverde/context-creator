use anyhow::Result;
use clap::Parser;
use code_digest::{cli::Config, run};

fn main() -> Result<()> {
    // Parse command line arguments
    let mut config = Config::parse();

    // Load configuration from file if specified
    config.load_from_file()?;

    // Run the application
    run(config)?;

    Ok(())
}
