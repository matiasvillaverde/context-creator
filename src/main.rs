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

    // Load configuration from file if specified (but not in MCP mode)
    if !config.mcp {
        config.load_from_file()?;
    }

    // Validate configuration first
    config.validate()?;

    // Check if MCP server mode is requested
    if config.mcp || config.rmcp {
        // Initialize logging for MCP server
        context_creator::logging::init_logging(&config)?;

        // Start MCP server - this runs forever
        run_mcp_server(config)?;
        return Ok(());
    }

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

#[tokio::main]
async fn run_mcp_server(config: Config) -> Result<()> {
    use context_creator::mcp_server;

    // Configure Rayon thread pool to use fewer threads to avoid competing with Tokio
    // Use half the available CPU cores for Rayon, minimum 1
    let rayon_threads = std::cmp::max(1, num_cpus::get() / 2);
    rayon::ThreadPoolBuilder::new()
        .num_threads(rayon_threads)
        .thread_name(|i| format!("rayon-worker-{i}"))
        .build_global()
        .ok(); // Ignore error if already initialized

    eprintln!("Configured Rayon thread pool with {rayon_threads} threads");

    // Use RMCP implementation if requested
    if config.rmcp {
        match config.rmcp_transport.as_str() {
            "stdio" => {
                eprintln!("Starting RMCP MCP server (stdio transport)");
                mcp_server::rmcp_server::transport::start_stdio().await?;
            }
            "http" => {
                let addr = format!("127.0.0.1:{}", config.mcp_port);
                eprintln!("Starting RMCP MCP server (HTTP/SSE transport) on {addr}");
                mcp_server::rmcp_server::transport::start_http(&addr).await?;
            }
            _ => {
                anyhow::bail!("Invalid RMCP transport mode: {}", config.rmcp_transport);
            }
        }
    } else {
        // Use original jsonrpsee implementation
        let addr = format!("127.0.0.1:{}", config.mcp_port);
        eprintln!("MCP server listening on {addr}");

        let handle = mcp_server::start_server(&addr).await?;

        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;
        eprintln!("Shutting down MCP server...");

        handle.stop()?;
    }

    Ok(())
}
