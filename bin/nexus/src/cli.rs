use primitives::Config;
use tasks::{monitor::MonitorTask, server::ServerTask, spawn_tasks};
use clap::Parser;
use toml::from_str;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliConfig {
    #[arg(short, long)]
    pub config_path: String,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    setup()?;
    let config = CliConfig::parse();
    let config_str = std::fs::read_to_string(&config.config_path)?;
    let config: Config = from_str(&config_str)?;

    // server config
    let server_config = config.clone().server;
    // monitor config
    let monitor_configs = config.clone().monitor;

    tracing::info!("Starting Chronicle with config: {:?}", config.clone());

    let mut tasks = vec![ServerTask::new(server_config).boxed()];

    for monitor_config in monitor_configs {
        tasks.push(MonitorTask::new(monitor_config).boxed());
    }

    spawn_tasks(tasks, tokio::signal::ctrl_c()).await;

    Ok(())
}

pub fn setup() -> Result<(), anyhow::Error> {
    let filter =
        tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .finish()
        .try_init()?;

    Ok(())
}
