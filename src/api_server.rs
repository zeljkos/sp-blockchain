use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower::ServiceBuilder;
use tokio::net::TcpListener;
use log::info;

use crate::bce_settlement::{BceRecord, Settlement, SettlementProcessor};
use std::sync::Arc;

#[derive(Clone)]
pub struct BceApiServer {
    host: String,
    port: u16,
    settlement_processor: Arc<SettlementProcessor>,
}

#[derive(Serialize, Deserialize)]
pub struct BceRecordSubmission {
    pub record: BceRecord,
    pub operator_signature: Option<String>,
}

#[derive(Serialize)]
pub struct SubmissionResponse {
    pub success: bool,
    pub message: String,
    pub settlement: Option<Settlement>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_info: String,
    pub timestamp: u64,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub settlement_stats: HashMap<String, u64>,
    pub api_info: HashMap<String, String>,
}

impl BceApiServer {
    pub fn new(
        host: String,
        port: u16,
        settlement_processor: Arc<SettlementProcessor>,
    ) -> Self {
        Self {
            host,
            port,
            settlement_processor,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.create_routes();

        let bind_addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&bind_addr).await?;

        info!("SP BCE API Server listening on {}", bind_addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    fn create_routes(&self) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/api/v1/bce/submit", post(submit_bce_record))
            .route("/api/v1/bce/stats", get(get_stats))
            .route("/api/v1/settlements", get(list_settlements))
            .route("/api/v1/settlements/:settlement_id", get(get_settlement))
            .route("/api/v1/network/status", get(get_network_status))
            .route("/api/v1/consensus/info", get(get_consensus_info))
            .with_state((*self.settlement_processor).clone())
            .layer(ServiceBuilder::new())
    }
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        node_info: "SP BCE Blockchain Node".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

async fn submit_bce_record(
    State(processor): State<SettlementProcessor>,
    Json(submission): Json<BceRecordSubmission>,
) -> Result<Json<SubmissionResponse>, StatusCode> {
    info!("Received BCE record submission: {}", submission.record.record_id);

    match processor.process_bce_record(submission.record).await {
        Ok(settlement) => {
            let message = if settlement.is_some() {
                "BCE record processed and settlement created".to_string()
            } else {
                "BCE record processed - settlement pending".to_string()
            };

            Ok(Json(SubmissionResponse {
                success: true,
                message,
                settlement,
            }))
        }
        Err(e) => {
            log::error!("Failed to process BCE record: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_stats(
    State(processor): State<SettlementProcessor>,
) -> Json<StatsResponse> {
    let settlement_stats = processor.get_settlement_stats().await;

    let mut api_info = HashMap::new();
    api_info.insert("version".to_string(), "0.1.0".to_string());
    api_info.insert("service".to_string(), "SP BCE Settlement Processor".to_string());

    Json(StatsResponse {
        settlement_stats,
        api_info,
    })
}

async fn list_settlements(
    State(processor): State<SettlementProcessor>,
) -> Result<Json<Vec<Settlement>>, StatusCode> {
    match processor.list_all_settlements().await {
        Ok(settlements) => Ok(Json(settlements)),
        Err(e) => {
            info!("Error listing settlements: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_settlement(
    Path(settlement_id): Path<String>,
    State(processor): State<SettlementProcessor>,
) -> Result<Json<Settlement>, StatusCode> {
    match processor.get_settlement_by_id(&settlement_id).await {
        Ok(Some(settlement)) => Ok(Json(settlement)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            info!("Error retrieving settlement {}: {}", settlement_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_network_status() -> Json<serde_json::Value> {
    // In real implementation, this would query the P2P network
    Json(serde_json::json!({
        "network_status": "active",
        "connected_peers": 2,
        "validator_peers": 2,
        "pending_settlements": 0,
        "node_type": "validator",
        "network_id": "sp-bce-testnet"
    }))
}

async fn get_consensus_info() -> Json<serde_json::Value> {
    // In real implementation, this would query the consensus manager
    Json(serde_json::json!({
        "consensus_state": "active",
        "is_validator": true,
        "current_epoch": 123,
        "block_height": 456,
        "finalized_settlements": 78,
        "validator_set_size": 3
    }))
}