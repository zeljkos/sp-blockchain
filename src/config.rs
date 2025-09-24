use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub api_host: String,
    pub api_port: u16,
    pub node_id: String,
    pub p2p_port: u16,
    pub settlement_threshold_eur: f64,
    pub validator_key_path: Option<PathBuf>,
    pub bootstrap_peers: Vec<String>,
}