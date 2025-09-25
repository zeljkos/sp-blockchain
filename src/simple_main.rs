use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio;
use log::{info, error};

use sp_blockchain::simple_blockchain::{SimpleBlockchain, BceRecord};

#[derive(Parser)]
#[command(name = "sp-bce-node")]
#[command(about = "Simple SP BCE Blockchain Node")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        #[arg(long, default_value = "./data")]
        data_dir: PathBuf,
        #[arg(long, default_value = "8080")]
        api_port: u16,
        #[arg(long, default_value = "0.0.0.0")]
        api_host: String,
        #[arg(long, default_value = "sp-node")]
        node_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    node_id: String,
    total_blocks: usize,
    total_records: u32,
    pending_records: usize,
}

struct AppState {
    blockchain: Arc<SimpleBlockchain>,
    node_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            data_dir,
            api_port,
            api_host,
            node_id,
        } => {
            start_node(data_dir, api_port, api_host, node_id).await?;
        }
    }

    Ok(())
}

async fn start_node(
    data_dir: PathBuf,
    api_port: u16,
    api_host: String,
    node_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Simple SP BCE Node: {}", node_id);
    println!("💾 Data Directory: {}", data_dir.display());
    println!("🌐 API: {}:{}", api_host, api_port);

    // Initialize simple blockchain (use default P2P port for simple mode)
    let (blockchain, _network_rx) = SimpleBlockchain::new(
        &data_dir.to_string_lossy(),
        node_id.clone(),
        30303
    ).await?;
    let blockchain = Arc::new(blockchain);

    println!("✅ Simple Blockchain initialized successfully");

    // Show existing storage contents
    blockchain.show_storage()?;

    // Create app state
    let state = AppState {
        blockchain,
        node_id,
    };

    // Build API routes
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/bce/submit", post(submit_bce_record))
        .route("/api/v1/bce/stats", get(get_stats))
        .route("/api/v1/blockchain/blocks", get(get_blocks))
        .route("/api/v1/storage/list", get(list_storage))
        .with_state(Arc::new(state));

    // Start API server
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", api_host, api_port)).await?;
    println!("🎯 API server listening on {}:{}", api_host, api_port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler(
    State(state): State<Arc<AppState>>
) -> Result<Json<HealthResponse>, StatusCode> {
    let stats = state.blockchain.get_stats().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = HealthResponse {
        status: "healthy".to_string(),
        node_id: state.node_id.clone(),
        total_blocks: stats.total_blocks,
        total_records: stats.total_records,
        pending_records: stats.pending_records,
    };

    Ok(Json(response))
}

async fn submit_bce_record(
    State(state): State<Arc<AppState>>,
    Json(record): Json<BceRecord>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("📝 Received BCE record submission: {}", record.record_id);

    match state.blockchain.submit_bce_record(record).await {
        Ok(record_id) => {
            info!("✅ BCE record processed successfully: {}", record_id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(record_id),
                message: "BCE record stored successfully".to_string(),
            }))
        }
        Err(e) => {
            error!("❌ Failed to process BCE record: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Failed to process BCE record: {}", e),
            }))
        }
    }
}

async fn get_stats(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let stats = state.blockchain.get_stats().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = serde_json::json!({
        "total_blocks": stats.total_blocks,
        "total_records": stats.total_records,
        "pending_records": stats.pending_records,
        "total_settlement_amount_cents": stats.total_settlement_amount_cents,
        "total_settlement_amount_eur": stats.total_settlement_amount_cents as f64 / 100.0,
        "last_block_time": stats.last_block_time,
    });

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response),
        message: "Statistics retrieved successfully".to_string(),
    }))
}

async fn get_blocks(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, StatusCode> {
    let blocks = state.blockchain.get_all_blocks().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let block_summaries: Vec<serde_json::Value> = blocks.iter().map(|block| {
        serde_json::json!({
            "block_number": block.block_number,
            "block_hash": format!("{}", block.block_hash),
            "timestamp": block.timestamp,
            "records_count": block.record_count,
            "total_amount_cents": block.settlement_summary.total_amount_cents,
            "total_amount_eur": block.settlement_summary.total_amount_cents as f64 / 100.0,
            "operator_balances": block.settlement_summary.operator_balances,
        })
    }).collect();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(block_summaries),
        message: "Blocks retrieved successfully".to_string(),
    }))
}

async fn list_storage(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    state.blockchain.show_storage().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some("Check server logs for storage contents".to_string()),
        message: "Storage contents listed in logs".to_string(),
    }))
}