use chronicle_primitives::Config;
use chronicle_tasks::{indexer::IndexerTask, server::ServerTask, spawn_tasks};
use clap::Parser;
use toml::from_str;
use tracing::{error, info, warn, debug};
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = std::fs::read_to_string(config_path)
        .map_err(|e| {
            error!("Failed to read configuration file: {}", e);
            e
        })?;

    debug!("Configuration file read successfully, parsing...");

    let config: Config = from_str(&config_str).map_err(|e| {
        error!("Failed to parse configuration file: {}", e);
        e
    })?;

    // Validate configuration
    validate_config(&config)?;

    info!(
        indexers_count = config.indexer.len(),
        "Configuration loaded and validated successfully"
    );

    Ok(config)
}

pub fn validate_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Check for duplicate event names
    let mut event_names = std::collections::HashSet::new();
    for indexer in &config.indexer {
        if !event_names.insert(&indexer.event_name) {
            return Err(format!(
                "Duplicate event name found: '{}'. Event names must be unique.",
                indexer.event_name
            ).into());
        }
    }

    // Validate database configuration
    if config.database.host.is_empty() {
        return Err("Database host cannot be empty".into());
    }

    if config.database.database.is_empty() {
        return Err("Database name cannot be empty".into());
    }

    debug!("Configuration validation passed");
    Ok(())
}


//==============================


pub fn setup_logging() -> anyhow::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .from_env_lossy();

    let fmt_layer = fmt::layer()
        .with_file(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_ansi(atty::is(atty::Stream::Stdout));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to init logger: {}", e))?;

    Ok(())
}

pub fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Application panicked: {}", panic_info);

        if let Some(location) = panic_info.location() {
            error!(
                "Panic occurred in file '{}' at line {}",
                location.file(),
                location.line(),
            );
        }

        eprintln!("Chronicle encountered a fatal error and needs to exit.");
        eprintln!("Please check the logs for more details.");
    }));
}

pub fn handle_user_friendly_error(err: &Box<dyn std::error::Error>) {
    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
        match io_err.kind() {
            std::io::ErrorKind::NotFound => {
                eprintln!("Error: Configuration file not found");
                eprintln!("Please check that the file exists at the specified path.");
            }
            std::io::ErrorKind::PermissionDenied => {
                eprintln!("Error: Permission denied");
                eprintln!("Please check file permissions for the configuration file.");
            }
            _ => {
                eprintln!("I/O Error: {}", io_err);
            }
        }
    } else if let Some(toml_err) = err.downcast_ref::<toml::de::Error>() {
        eprintln!("Configuration Error: {}", toml_err.message());
        eprintln!("Please check your configuration file for syntax errors.");
    } else if let Some(db_err) = err.downcast_ref::<tokio_postgres::Error>() {
        eprintln!("Database Error: {}", db_err);
        if db_err.to_string().contains("does not exist") {
            eprintln!("Please ensure the database exists and connection details are correct.");
        }
    } else {
        eprintln!("Error: {}", err);
    }

    eprintln!("\nFor more details, check the application logs.");
}

