use serde::{Deserialize, Serialize};

pub mod db;
pub mod error;
pub mod interfaces;
pub mod monitor;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub name: Option<String>,
    pub indexer: Vec<IndexerConfig>,
    pub server: ServerConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub db_url: String,
    pub server_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexerConfig {
    pub event_name: String,
    pub rpc_url: String,
    pub address: String,
    pub event_signature: String,
    pub block_number: u64,
    pub db_url: String,
}
