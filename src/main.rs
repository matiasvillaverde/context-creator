use anyhow::Result;
use clap::Parser;
use context_creator::{cli::Config, run};

fn main() -> Result<()> {
    // Parse command line arguments
    let mut config = Config::parse();

    // Load configuration from file if specified
    config.load_from_file()?;

    // Read prompt from stdin if needed
    if config.should_read_stdin() {
        use std::io::Read;
        let mut buffer = String::new();
        let _ = std::io::stdin().read_to_string(&mut buffer)?;

        // Set the prompt from stdin if not already set
        if config.prompt.is_none() {
            config.prompt = Some(buffer.trim().to_string());
        }
    }

    // Run the application
    run(config)?;

    Ok(())
}
