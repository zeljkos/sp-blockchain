pub mod p2p;
pub mod consensus;

use crate::hash::Blake2bHash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // Block propagation
    NewBlock {
        block_hash: Blake2bHash,
        block_data: Vec<u8>,
    },
    RequestBlock {
        block_hash: Blake2bHash,
    },
    BlockResponse {
        block_hash: Blake2bHash,
        block_data: Option<Vec<u8>>,
    },

    // Consensus voting
    Vote {
        block_hash: Blake2bHash,
        validator_id: String,
        signature: Vec<u8>,
        approve: bool,
    },

    // Chain synchronization
    RequestChainState,
    ChainStateResponse {
        height: u64,
        head_hash: Blake2bHash,
        known_blocks: Vec<Blake2bHash>,
    },

    // Peer discovery
    Ping,
    Pong,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: String,
    pub peer_id: libp2p::PeerId,
    pub addresses: Vec<libp2p::Multiaddr>,
    pub last_seen: std::time::SystemTime,
}

pub type NetworkResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;