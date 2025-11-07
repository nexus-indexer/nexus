pub mod db;
pub mod error;
pub mod monitor;
pub mod traits;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub name: Option<String>,
    pub monitor: Vec<MonitorConfig>,
    pub server: ServerConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub db_url: String,
    pub server_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MonitorConfig {
    pub event_name: String,
    pub rpc_url: String,
    pub address: String,
    pub event_signature: String,
    pub block_number: u64,
    pub db_url: String,
}
