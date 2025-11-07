pub mod utils;

use chronicle_tasks::{indexer::IndexerTask, server::ServerTask, spawn_tasks};
use clap::Parser;
use tracing::{debug, error, info, warn};

use crate::cli::utils::{load_config, setup_logging, setup_panic_hook};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Chronicle - Blockchain Event Indexer",
    long_about = "A high-performance blockchain event indexer that stores and queries on-chain events efficiently."
)]
pub struct CliConfig {
    #[arg(short, long, help = ".config.toml", default_value = ".config.toml")]
    pub config_path: String,

    #[arg(long, help = "Enable debug logging", default_value = "false")]
    pub debug: bool,

    #[arg(long, help = "Enable trace logging (very verbose)", default_value = "false")]
    pub trace: bool,
}

/// Main entry point for the CLI
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging first to capture any startup errors
    if let Err(e) = setup_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    // Set up panic hook for better error reporting
    setup_panic_hook();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Chronicle Blockchain Indexer"
    );    
    
    
    let cli_config = CliConfig::parse();

    if cli_config.trace {
        std::env::set_var("RUST_LOG", "trace");
    } else if cli_config.debug {
        std::env::set_var("RUST_LOG", "debug");
    }

    // Logging is already set up in main.rs, so we don't set it up here
    debug!(
        config_path = %cli_config.config_path,
        debug_mode = cli_config.debug,
        trace_mode = cli_config.trace,
        "CLI configuration parsed"
    );

    info!(
        config_path = %cli_config.config_path,
        "Loading configuration"
    );

    // Load and validate configuration
    let config = load_config(&cli_config.config_path).await?;

    let server_config = config.server.clone();
    let indexer_configs = config.indexer.clone();

    info!(
        total_indexers = indexer_configs.len(),
        "Starting Chronicle with single database architecture"
    );

    // Create and spawn tasks
    let mut tasks = vec![ServerTask::new(server_config).boxed()];
    let mut successful_indexers = 0;

    for (index, indexer_config) in indexer_configs.into_iter().enumerate() {
        debug!(
            index = index,
            event_name = %indexer_config.event_name,
            contract_address = %indexer_config.address,
            "Creating indexer task"
        );

        match IndexerTask::new(indexer_config.clone()).await {
            Ok(task) => {
                tasks.push(task.clone().boxed());
                successful_indexers += 1;
                info!(
                    event_name = %task.config.event_name,
                    "Indexer task created successfully"
                );
            }
            Err(e) => {
                error!(
                    event_name = %indexer_config.event_name,
                    error = %e,
                    "Failed to create indexer task"
                );

                // Don't fail the entire application if one indexer fails
                // Just log and continue with others
                continue;
            }
        }
    }

    // Validate that we have at least some tasks running
    if successful_indexers == 0 {
        warn!("No indexer tasks were successfully created");
        if tasks.len() == 1 {
            warn!("Only server task will run - no events will be indexed");
        }
    } else {
        info!(
            successful_indexers,
            total_tasks = tasks.len(),
            "All tasks created successfully"
        );
    }

    // Spawn all tasks with graceful shutdown handling
    info!("Spawning all tasks...");
    spawn_tasks(tasks, tokio::signal::ctrl_c()).await;

    info!("Chronicle shutdown complete");
    Ok(())
}
