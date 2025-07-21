use anyhow::Result;
use clap::Parser;
use context_creator::{cli::Config, run};

fn main() {
    // Use a custom error handler to provide clean error messages
    if let Err(err) = run_main() {
        // Print error message without the backtrace
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<()> {
    // Parse command line arguments
    let mut config = Config::parse();

    // Load configuration from file if specified
    config.load_from_file()?;

    // Initialize logging based on configuration
    context_creator::logging::init_logging(&config)?;

    // Read prompt from stdin if needed
    if config.should_read_stdin() {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;

        // Set the prompt from stdin if not already set
        if config.prompt.is_none() {
            config.prompt = Some(buffer.trim().to_string());
        }
    }

    // Run the application
    run(config)?;

    Ok(())
}
