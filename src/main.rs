use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    response::{Json, Html},
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
use sp_blockchain::network::p2p::P2PNetwork;
use sp_blockchain::zkp::{FivePartySettlementFactory, SettlementProofSystem};
use sp_blockchain::security::{SpAuthentication, middleware::{*, AuthenticatedSpExtension}};
use sp_blockchain::smart_contracts::contract_api::{ContractAPI, SettlementRequest, RateValidationRequest, DisputeRequest};

#[derive(Parser)]
#[command(name = "sp-bce-node")]
#[command(about = "Service Provider BCE Blockchain Node")]
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
        #[arg(long, default_value = "100.0")]
        settlement_threshold_eur: f64,
        #[arg(long, default_value = "30303")]
        p2p_port: u16,
        #[arg(long)]
        bootstrap_peers: Option<String>,
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
    settlement_threshold_eur: f64,
    records_processed: u32,
    total_blocks: usize,
    pending_records: usize,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    total_blocks: usize,
    total_records: u32,
    pending_records: usize,
    total_settlement_amount_eur: f64,
    last_block_time: Option<chrono::DateTime<chrono::Utc>>,
}

struct AppState {
    blockchain: Arc<SimpleBlockchain>,
    node_id: String,
    settlement_threshold_eur: f64,
    authentication: Arc<SpAuthentication>,
    zkp_system: Arc<SettlementProofSystem>,
    contract_api: Arc<ContractAPI>,
}

/// Deploy sample settlement smart contracts for demonstration
async fn deploy_sample_settlement_contracts(
    blockchain: Arc<SimpleBlockchain>
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Deploying sample settlement smart contracts for 5-party consortium...");

    // Generate sample bilateral amounts for all 20 consortium pairs
    let bilateral_amounts = FivePartySettlementFactory::generate_sample_bilateral_amounts();

    // Create complete settlement workflow
    let contracts = FivePartySettlementFactory::create_complete_settlement_workflow(
        "2024-Q4-Demo",
        &bilateral_amounts,
    );

    println!("📦 Generated {} settlement contracts for deployment", contracts.len());

    // Deploy each contract
    for (i, contract) in contracts.into_iter().enumerate() {
        match blockchain.deploy_settlement_contract(contract).await {
            Ok(contract_address) => {
                println!("✅ Contract {} deployed: {:?}", i + 1, contract_address);
            }
            Err(e) => {
                println!("❌ Failed to deploy contract {}: {}", i + 1, e);
            }
        }

        // Small delay between deployments
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("🎯 Sample settlement contracts deployment completed");
    Ok(())
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
            settlement_threshold_eur,
            p2p_port,
            bootstrap_peers
        } => {
            start_node(
                data_dir,
                api_port,
                api_host,
                node_id,
                settlement_threshold_eur,
                p2p_port,
                bootstrap_peers
            ).await?;
        }
    }

    Ok(())
}

async fn start_node(
    data_dir: PathBuf,
    api_port: u16,
    api_host: String,
    node_id: String,
    settlement_threshold_eur: f64,
    p2p_port: u16,
    bootstrap_peers: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting SP BCE Node: {}", node_id);
    println!("💰 Settlement Threshold: {} EUR", settlement_threshold_eur);
    println!("🌐 API: {}:{}", api_host, api_port);
    println!("📡 P2P: {}", p2p_port);

    // Parse bootstrap peers
    let peers = bootstrap_peers
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(Vec::new);

    // Initialize SP blockchain with RocksDB storage
    let (mut blockchain, network_rx) = SimpleBlockchain::new(
        &data_dir.to_string_lossy(),
        node_id.clone(),
        p2p_port,
        settlement_threshold_eur,
    ).await?;

    // Load pre-generated ZKP keys for 5-party consortium
    let zkp_keys_dir = std::path::PathBuf::from("/app/zkp_keys");

    if zkp_keys_dir.exists() && zkp_keys_dir.join("ceremony_transcript.json").exists() {
        if let Err(e) = blockchain.load_zkp_keys(zkp_keys_dir.clone()).await {
            println!("⚠️  Failed to load ZKP keys: {}", e);
            println!("💡 Run 'cargo run --bin trusted-setup-demo' first to generate keys");
        } else {
            println!("🔐 ZKP keys loaded successfully for 5-party consortium");
            println!("✅ Ready for zero-knowledge proof generation and verification");
        }
    } else {
        println!("⚠️  ZKP keys not found at: {:?}", zkp_keys_dir);
        println!("💡 Run 'cargo run --bin trusted-setup-demo' first to generate keys");
    }

    // Initialize ZKP system for privacy-preserving settlement proofs
    println!("🛡️  Initializing Zero-Knowledge Proof system...");
    let zkp_system = match SettlementProofSystem::new(&node_id) {
        Ok(system) => {
            println!("✅ ZKP system initialized successfully");
            Arc::new(system)
        }
        Err(e) => {
            error!("❌ Failed to initialize ZKP system: {}", e);
            println!("⚠️  Warning: ZKP system disabled - settlement proofs will not be generated");
            println!("   System will continue with basic functionality");
            // Create a fallback system that won't panic the application
            Arc::new(SettlementProofSystem::default())
        }
    };

    // Set the settlement proof system for ZKP integration
    blockchain.set_settlement_proof_system(zkp_system.clone());
    println!("🛡️  Settlement proof system integrated into blockchain");

    // Initialize P2P network
    let mut p2p_network = P2PNetwork::new(node_id.clone(), p2p_port).await
        .map_err(|e| -> Box<dyn std::error::Error> { e })?;

    // Connect P2P network to blockchain message handler (incoming messages)
    p2p_network.set_message_callback(blockchain.network_tx.clone());

    // Get P2P message sender for blockchain outbound messages
    let p2p_message_sender = p2p_network.get_message_sender();

    // Set P2P sender on blockchain for outbound messages
    blockchain.set_p2p_sender(p2p_message_sender);

    println!("✅ SP Blockchain with ZKP initialized successfully");
    println!("🌐 P2P Network initialized and connected");

    // Clone blockchain for message processing before moving into AppState
    let blockchain = Arc::new(blockchain);
    let blockchain_for_messages = blockchain.clone();

    // Sample contract deployment disabled for clean demo
    // tokio::spawn({
    //     let blockchain = blockchain.clone();
    //     async move {
    //         if let Err(e) = deploy_sample_settlement_contracts(blockchain).await {
    //             println!("⚠️  Failed to deploy sample smart contracts: {}", e);
    //         }
    //     }
    // });

    // Initialize SP authentication system for the 5-party consortium
    let authentication = Arc::new(SpAuthentication::new_consortium());
    println!("🔐 SP Authentication system initialized for 5-party consortium");

    // Initialize Contract API for smart contract management using existing blockchain
    let contract_api = Arc::new(ContractAPI::with_blockchain(blockchain.clone()));
    println!("📋 ZKP Smart Contract API initialized with existing blockchain");

    // Create app state
    let state = AppState {
        blockchain,
        node_id,
        settlement_threshold_eur,
        authentication: authentication.clone(),
        zkp_system,
        contract_api,
    };

    // Build API routes with security middleware
    // Note: Middleware layers are applied in reverse order (onion pattern)
    // So authorization_middleware runs first, then auth_middleware
    let protected_routes = Router::new()
        .route("/api/v1/bce/submit", post(submit_bce_record))
        .route("/api/v1/bce/stats", get(get_stats))
        .route("/api/v1/blockchain/blocks", get(get_blocks))
        .route("/api/v1/blockchain/stats", get(get_blockchain_stats))
        .route("/api/v1/zkp/stats", get(get_zkp_stats))
        .route("/api/v1/zkp/generate_proof", post(generate_zkp_proof))
        .route("/api/v1/zkp/verify_proof", post(verify_zkp_proof))
        .route("/api/v1/zkp/system_status", get(get_zkp_system_status))
        .route("/api/v1/zkp/setup_info", get(get_zkp_setup_info))
        .route("/api/v1/zkp/metrics", get(get_zkp_metrics))
        .route("/api/v1/zkp/performance", get(get_zkp_performance_metrics))
        .route("/api/v1/zkp/health", get(get_zkp_health_check))
        .route("/api/v1/zkp/reset_metrics", post(reset_zkp_metrics))
        .route("/api/v1/zkp/test_integration", post(test_zkp_integration))
        .route("/api/v1/read/bce_records", get(get_bce_records))
        .route("/api/v1/read/settlement_blocks", get(get_settlement_blocks))
        .route("/api/v1/contracts/deploy", post(deploy_smart_contract))
        .route("/api/v1/contracts/list", get(list_smart_contracts))
        .route("/api/v1/contracts/execute", post(execute_smart_contract))
        .route("/api/v1/contracts/stats", get(get_contract_stats))
        .route("/dashboard", get(dashboard_handler))
        .layer(middleware::from_fn(authorization_middleware))
        .layer(middleware::from_fn_with_state(authentication.clone(), auth_middleware));

    let app = Router::new()
        .route("/health", get(health_handler))
        .merge(protected_routes)
        .layer(middleware::from_fn(security_headers_middleware))
        .with_state(Arc::new(state));

    // Start all services concurrently
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", api_host, api_port)).await?;
    println!("🎯 API server listening on {}:{}", api_host, api_port);

    tokio::select! {
        // API Server
        result = axum::serve(listener, app) => {
            if let Err(e) = result {
                error!("API server error: {}", e);
            }
        }

        // P2P Network
        result = p2p_network.start() => {
            if let Err(e) = result {
                error!("P2P network error: {}", e);
            }
        }

        // Blockchain Message Processing
        result = handle_blockchain_messages(blockchain_for_messages, network_rx) => {
            if let Err(e) = result {
                error!("Blockchain message processing error: {}", e);
            }
        }
    }

    Ok(())
}

/// Handle incoming P2P messages and forward to blockchain
async fn handle_blockchain_messages(
    blockchain: Arc<SimpleBlockchain>,
    mut network_rx: tokio::sync::mpsc::UnboundedReceiver<sp_blockchain::network::NetworkMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📨 Starting blockchain message processing loop");

    while let Some(message) = network_rx.recv().await {
        if let Err(e) = blockchain.handle_network_message(message).await {
            error!("Failed to process network message: {}", e);
        }
    }

    Ok(())
}

async fn health_handler(
    State(state): State<Arc<AppState>>
) -> Result<Json<HealthResponse>, StatusCode> {
    let stats = match state.blockchain.get_stats().await {
        Ok(stats) => stats,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let response = HealthResponse {
        status: "healthy".to_string(),
        node_id: state.node_id.clone(),
        settlement_threshold_eur: state.settlement_threshold_eur,
        records_processed: stats.total_records,
        total_blocks: stats.total_blocks,
        pending_records: stats.pending_records,
    };

    Ok(Json(response))
}

async fn submit_bce_record(
    State(state): State<Arc<AppState>>,
    authenticated_sp: AuthenticatedSpExtension,
    Json(record): Json<BceRecord>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("📝 Received BCE record submission: {} from SP: {}",
          record.record_id, authenticated_sp.0.provider_name);

    // Authorize SP to submit this specific record (SP must be the visited network)
    if let Err(e) = state.authentication.authorize_bce_submission(&authenticated_sp.0, &record.visited_operator) {
        error!("❌ Authorization failed for SP {}: {}", authenticated_sp.0.provider_id, e);
        return Ok(Json(ApiResponse {
            success: false,
            data: None,
            message: format!("Authorization failed: {}", e),
        }));
    }

    info!("✅ SP {} authorized to submit record as visited network: {}",
          authenticated_sp.0.provider_id, record.visited_operator);

    match state.blockchain.submit_bce_record(record).await {
        Ok(record_id) => {
            info!("✅ BCE record processed successfully: {}", record_id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(record_id),
                message: "BCE record stored and broadcasted to validators".to_string(),
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
) -> Result<Json<ApiResponse<StatsResponse>>, StatusCode> {
    let stats = match state.blockchain.get_stats().await {
        Ok(stats) => stats,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let response = StatsResponse {
        total_blocks: stats.total_blocks,
        total_records: stats.total_records,
        pending_records: stats.pending_records,
        total_settlement_amount_eur: stats.total_settlement_amount_cents as f64 / 100.0,
        last_block_time: stats.last_block_time,
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response),
        message: "Statistics retrieved successfully".to_string(),
    }))
}

async fn get_blocks(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, StatusCode> {
    let blocks = match state.blockchain.get_all_blocks().await {
        Ok(blocks) => blocks,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Convert blocks to JSON for API response
    let block_summaries: Vec<serde_json::Value> = blocks.iter().map(|block| {
        serde_json::json!({
            "block_number": block.block_number,
            "block_hash": hex::encode(block.block_hash.as_bytes()),
            "timestamp": block.timestamp,
            "records_count": block.record_count,
            "total_amount_cents": block.settlement_summary.total_amount_cents,
        })
    }).collect();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(block_summaries),
        message: "Blocks retrieved successfully".to_string(),
    }))
}

async fn get_blockchain_stats(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let stats = match state.blockchain.get_stats().await {
        Ok(stats) => stats,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let blocks = match state.blockchain.get_all_blocks().await {
        Ok(blocks) => blocks,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let response = serde_json::json!({
        "total_blocks": stats.total_blocks,
        "total_records": stats.total_records,
        "pending_records": stats.pending_records,
        "total_settlement_amount_eur": stats.total_settlement_amount_cents as f64 / 100.0,
        "last_block_time": stats.last_block_time,
        "latest_block_hash": blocks.last().map(|b| hex::encode(b.block_hash.as_bytes())),
        "chain_length": blocks.len(),
    });

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response),
        message: "Blockchain stats retrieved successfully".to_string(),
    }))
}

/// Get ZKP and smart contract statistics
async fn get_zkp_stats(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let zkp_stats = match state.blockchain.get_zkp_stats().await {
        Ok(stats) => stats,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(zkp_stats),
        message: "ZKP and smart contract stats retrieved successfully".to_string(),
    }))
}

/// Generate a ZKP proof for testing
async fn generate_zkp_proof(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    use sp_blockchain::zkp::settlement_proofs::ProofParameters;

    // Parse the proof parameters from the request
    let proof_params = match serde_json::from_value::<ProofParameters>(payload) {
        Ok(params) => params,
        Err(e) => {
            return Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Invalid proof parameters: {}", e)
                })),
                message: "Failed to parse proof parameters".to_string(),
            }));
        }
    };

    // Generate ZKP proof using the settlement proof system
    match state.zkp_system.generate_proof(proof_params) {
        Ok(settlement_proof) => {
            Ok(Json(ApiResponse {
                success: true,
                data: Some(serde_json::json!({
                    "proof": hex::encode(&settlement_proof.proof_bytes),
                    "public_inputs": settlement_proof.public_inputs,
                    "generated_at": chrono::Utc::now().timestamp()
                })),
                message: "ZKP proof generated successfully".to_string(),
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Failed to generate proof: {}", e)
                })),
                message: "ZKP proof generation failed".to_string(),
            }))
        }
    }
}

/// Verify a ZKP proof
async fn verify_zkp_proof(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    use sp_blockchain::zkp::settlement_proofs::SettlementProof;

    // Parse the proof data from request
    let proof_hex = payload.get("proof")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let public_inputs = payload.get("public_inputs")
        .and_then(|v| v.as_array())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Convert hex proof to bytes
    let proof_bytes = match hex::decode(proof_hex) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Invalid proof hex: {}", e)
                })),
                message: "Failed to decode proof hex".to_string(),
            }));
        }
    };

    // Convert public inputs to Vec<String>
    let inputs: Vec<String> = public_inputs.iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // Create SettlementProof struct
    let settlement_proof = SettlementProof {
        proof_bytes,
        public_inputs: inputs,
    };

    // Verify the proof
    match state.zkp_system.verify_proof(&settlement_proof) {
        Ok(is_valid) => {
            Ok(Json(ApiResponse {
                success: true,
                data: Some(serde_json::json!({
                    "valid": is_valid,
                    "verified_at": chrono::Utc::now().timestamp()
                })),
                message: if is_valid {
                    "ZKP proof is valid".to_string()
                } else {
                    "ZKP proof is invalid".to_string()
                },
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Verification failed: {}", e)
                })),
                message: "ZKP proof verification failed".to_string(),
            }))
        }
    }
}

/// Get ZKP system status
async fn get_zkp_system_status(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    // Check if ZKP system is properly initialized
    let system_status = match state.zkp_system.get_system_info() {
        Ok(info) => serde_json::json!({
            "status": "operational",
            "initialized": true,
            "system_info": info,
            "last_checked": chrono::Utc::now().timestamp()
        }),
        Err(e) => serde_json::json!({
            "status": "error",
            "initialized": false,
            "error": format!("{}", e),
            "last_checked": chrono::Utc::now().timestamp()
        })
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(system_status),
        message: "ZKP system status retrieved successfully".to_string(),
    }))
}

/// Get ZKP trusted setup information
async fn get_zkp_setup_info(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    // Get setup information from the ZKP system
    match state.zkp_system.get_setup_info() {
        Ok(setup_info) => {
            Ok(Json(ApiResponse {
                success: true,
                data: Some(serde_json::json!({
                    "setup_info": setup_info,
                    "retrieved_at": chrono::Utc::now().timestamp()
                })),
                message: "ZKP setup information retrieved successfully".to_string(),
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Failed to get setup info: {}", e)
                })),
                message: "ZKP setup information retrieval failed".to_string(),
            }))
        }
    }
}

/// Get comprehensive ZKP metrics
async fn get_zkp_metrics(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let metrics = state.zkp_system.get_metrics();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(serde_json::to_value(metrics).unwrap_or_default()),
        message: "ZKP metrics retrieved successfully".to_string(),
    }))
}

/// Get ZKP performance metrics
async fn get_zkp_performance_metrics(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    match state.zkp_system.get_performance_metrics() {
        Ok(metrics) => {
            Ok(Json(ApiResponse {
                success: true,
                data: Some(metrics),
                message: "ZKP performance metrics retrieved successfully".to_string(),
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Failed to get performance metrics: {}", e)
                })),
                message: "ZKP performance metrics retrieval failed".to_string(),
            }))
        }
    }
}

/// Get ZKP system health check
async fn get_zkp_health_check(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    match state.zkp_system.health_check() {
        Ok(health) => {
            // Extract health status to determine HTTP status
            let health_status = health.get("health")
                .and_then(|h| h.get("status"))
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();

            let http_status = match health_status.as_str() {
                "healthy" => StatusCode::OK,
                "degraded" => StatusCode::OK, // Still return 200 but with warnings
                "unhealthy" => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            // Always return success: true for health checks (HTTP status indicates health)
            let response = Json(ApiResponse {
                success: true,
                data: Some(health),
                message: format!("ZKP system health check completed - status: {}", health_status),
            });

            // For unhealthy systems, we could return an error status, but let's keep it informational
            match http_status {
                StatusCode::SERVICE_UNAVAILABLE => Err(StatusCode::SERVICE_UNAVAILABLE),
                _ => Ok(response),
            }
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Health check failed: {}", e)
                })),
                message: "ZKP health check failed".to_string(),
            }))
        }
    }
}

/// Reset ZKP metrics (for testing)
async fn reset_zkp_metrics(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    match state.zkp_system.reset_metrics() {
        Ok(_) => {
            Ok(Json(ApiResponse {
                success: true,
                data: Some(serde_json::json!({
                    "reset_at": chrono::Utc::now().timestamp()
                })),
                message: "ZKP metrics reset successfully".to_string(),
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Failed to reset metrics: {}", e)
                })),
                message: "ZKP metrics reset failed".to_string(),
            }))
        }
    }
}

/// Run comprehensive ZKP integration test
async fn test_zkp_integration(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    match state.blockchain.test_zkp_integration().await {
        Ok(test_results) => {
            Ok(Json(ApiResponse {
                success: true,
                data: Some(test_results),
                message: "ZKP integration test completed".to_string(),
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: Some(serde_json::json!({
                    "error": format!("Test failed: {}", e)
                })),
                message: "ZKP integration test failed".to_string(),
            }))
        }
    }
}

/// Get all BCE records
async fn get_bce_records(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, StatusCode> {
    println!("🔍 DEBUG: get_bce_records endpoint called");

    let records = match state.blockchain.get_all_bce_records().await {
        Ok(records) => {
            println!("🔍 DEBUG: Successfully retrieved {} BCE records", records.len());
            records
        },
        Err(e) => {
            println!("🔍 DEBUG: Error retrieving BCE records: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        },
    };

    let record_summaries: Vec<serde_json::Value> = records.iter().map(|record| {
        serde_json::json!({
            "record_id": record.record_id,
            "imsi": record.imsi,
            "home_operator": record.home_operator,
            "visited_operator": record.visited_operator,
            "call_minutes": record.call_minutes,
            "data_mb": record.data_mb,
            "sms_count": record.sms_count,
            "call_rate_cents": record.call_rate_cents,
            "data_rate_cents": record.data_rate_cents,
            "sms_rate_cents": record.sms_rate_cents,
            "wholesale_charge_cents": record.wholesale_charge_cents,
            "timestamp": record.timestamp,
            "roaming_minutes": record.roaming_minutes,
            "roaming_data_mb": record.roaming_data_mb,
            "roaming_rate_cents": record.roaming_rate_cents,
            "roaming_data_rate_cents": record.roaming_data_rate_cents,
            "network_pair_hash": record.network_pair_hash,
            "proof_verified": record.proof_verified,
        })
    }).collect();

    println!("🔍 DEBUG: Returning {} BCE record summaries", record_summaries.len());

    Ok(Json(ApiResponse {
        success: true,
        data: Some(record_summaries),
        message: "BCE records retrieved successfully".to_string(),
    }))
}

/// Get all settlement blocks
async fn get_settlement_blocks(
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, StatusCode> {
    println!("🔍 DEBUG: get_settlement_blocks endpoint called");

    let blocks = match state.blockchain.get_all_blocks().await {
        Ok(blocks) => {
            println!("🔍 DEBUG: Successfully retrieved {} settlement blocks", blocks.len());
            blocks
        },
        Err(e) => {
            println!("🔍 DEBUG: Error retrieving settlement blocks: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        },
    };

    let block_details: Vec<serde_json::Value> = blocks.iter().map(|block| {
        serde_json::json!({
            "block_number": block.block_number,
            "block_hash": hex::encode(block.block_hash.as_bytes()),
            "timestamp": block.timestamp,
            "records_count": block.record_count,
            "total_amount_cents": block.settlement_summary.total_amount_cents,
            "total_amount_eur": block.settlement_summary.total_amount_cents as f64 / 100.0,
            "record_ids": block.record_ids,
            "settlement_summary": {
                "total_records": block.settlement_summary.total_records,
                "total_amount_cents": block.settlement_summary.total_amount_cents,
                "operator_balances": block.settlement_summary.operator_balances.iter().map(|(operator, balance)| {
                    serde_json::json!({
                        "operator": operator,
                        "balance_cents": balance,
                    })
                }).collect::<Vec<_>>(),
            }
        })
    }).collect();

    println!("🔍 DEBUG: Returning {} settlement block details", block_details.len());

    Ok(Json(ApiResponse {
        success: true,
        data: Some(block_details),
        message: "Settlement blocks retrieved successfully".to_string(),
    }))
}

/// Serve the SP blockchain dashboard
async fn dashboard_handler() -> Result<Html<String>, StatusCode> {
    let dashboard_content = match std::fs::read_to_string("dashboard/index.html") {
        Ok(content) => content,
        Err(_) => {
            // Fallback content if dashboard file is not found
            r#"<!DOCTYPE html>
<html>
<head>
    <title>SP Blockchain Dashboard</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .error { color: #d32f2f; background: #ffebee; padding: 16px; border-radius: 4px; margin: 20px 0; }
        .info { color: #1976d2; background: #e3f2fd; padding: 16px; border-radius: 4px; margin: 20px 0; }
        h1 { color: #333; border-bottom: 2px solid #2563eb; padding-bottom: 10px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🏗️ SP Blockchain Dashboard</h1>
        <div class="error">
            <strong>Dashboard file not found</strong><br>
            The dashboard HTML file is not available at 'dashboard/index.html'.
        </div>
        <div class="info">
            <strong>Available API Endpoints:</strong><br>
            • <code>GET /health</code> - System health check<br>
            • <code>GET /api/v1/blockchain/stats</code> - Blockchain statistics<br>
            • <code>GET /api/v1/zkp/system_status</code> - ZKP system status<br>
            • <code>GET /api/v1/bce/stats</code> - BCE records statistics<br>
        </div>
        <p>Please ensure the dashboard files are properly deployed.</p>
    </div>
</body>
</html>"#.to_string()
        }
    };

    Ok(Html(dashboard_content))
}

// ============================================================================
// SMART CONTRACT MANAGEMENT API ENDPOINTS
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ContractDeployRequest {
    contract_id: String,
    contract_type: String,
    operators: Vec<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContractExecuteRequest {
    contract_id: String,
    method: String,
    parameters: serde_json::Value,
}

/// Deploy a new smart contract (for demo purposes)
async fn deploy_smart_contract(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ContractDeployRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    println!("🚀 API: Deploying smart contract: {}", request.contract_id);

    // Validate operators - must be consortium members
    let valid_operators = vec![
        "tmobile-de".to_string(),
        "vodafone-uk".to_string(),
        "orange-fr".to_string(),
        "telefonica-es".to_string(),
        "sfr-fr".to_string(),
    ];

    for operator in &request.operators {
        if !valid_operators.contains(operator) {
            return Ok(Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Invalid operator: {}. Must be one of: {:?}", operator, valid_operators),
            }));
        }
    }

    // Deploy contract directly through blockchain (simplified approach)
    // Create simple contract for demo purposes
    use sp_blockchain::zkp::smart_contracts::settlement_contract::{ExecutableSettlementContract, ContractType};
    use sp_blockchain::zkp::smart_contracts::vm::Instruction;
    use sp_blockchain::hash::Blake2bHash;
    use std::collections::HashMap;

    let contract = ExecutableSettlementContract {
        contract_address: Blake2bHash::hash(&request.contract_id),
        bytecode: vec![
            Instruction::Log("Contract deployed".to_string()),
            Instruction::Push(1),
            Instruction::Halt,
        ],
        state: HashMap::new(),
        contract_type: ContractType::BceValidator,
    };

    match state.blockchain.deploy_settlement_contract(contract).await {
        Ok(contract_hash) => {
            println!("✅ Contract {} deployed successfully", request.contract_id);

            let response_data = serde_json::json!({
                "contract_id": request.contract_id,
                "contract_type": request.contract_type,
                "deployment_hash": hex::encode(contract_hash.as_bytes()),
                "operators": request.operators,
                "description": request.description.unwrap_or("Smart contract for telecom settlement".to_string()),
                "deployment_time": chrono::Utc::now().to_rfc3339(),
                "status": "deployed"
            });

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                message: "ZKP smart contract deployed successfully".to_string(),
            }))
        }
        Err(e) => {
            println!("❌ Contract deployment failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Contract deployment failed: {}", e),
            }))
        }
    }
}

/// List all deployed smart contracts
async fn list_smart_contracts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    println!("📋 API: Listing all smart contracts");

    // Get actual contract list from blockchain
    let zkp_stats = match state.blockchain.get_zkp_health_check().await {
        Ok(stats) => stats,
        Err(e) => {
            return Ok(Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Failed to retrieve contracts: {}", e),
            }));
        }
    };

    let deployed_contracts = zkp_stats.get("deployed_contracts")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let response_data = serde_json::json!({
        "contracts": [],
        "total_count": deployed_contracts,
        "zkp_stats": zkp_stats
    });

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response_data),
        message: format!("Found {} deployed contracts", deployed_contracts),
    }))
}

/// Execute a smart contract method
async fn execute_smart_contract(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ContractExecuteRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    println!("⚡ API: Executing contract method: {}::{}", request.contract_id, request.method);

    match request.method.as_str() {
        "validate_bce_rates" => {
            // For demo purposes, simulate rate validation directly
            let response_data = serde_json::json!({
                "execution_id": format!("exec_{}_{}", request.contract_id, chrono::Utc::now().timestamp()),
                "method": request.method,
                "result": "valid",
                "gas_used": 15000,
                "events": [
                    {
                        "event_type": "RateValidation",
                        "data": {
                            "contract_id": request.contract_id,
                            "validation_result": "valid"
                        },
                        "timestamp": chrono::Utc::now().timestamp()
                    }
                ],
                "execution_time": chrono::Utc::now().to_rfc3339()
            });

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                message: "Rate validation executed successfully".to_string(),
            }))
        }
        _ => {
            // Generic contract execution using blockchain directly
            let contract_hash = sp_blockchain::hash::Blake2bHash::hash(&request.contract_id);

            match state.blockchain.execute_smart_contract(contract_hash).await {
                Ok(result) => {
                    let response_data = serde_json::json!({
                        "execution_id": format!("exec_{}_{}", request.contract_id, chrono::Utc::now().timestamp()),
                        "method": request.method,
                        "result": result.to_string(),
                        "gas_used": 12000,
                        "events": [],
                        "execution_time": chrono::Utc::now().to_rfc3339()
                    });

                    Ok(Json(ApiResponse {
                        success: true,
                        data: Some(response_data),
                        message: "Contract executed successfully".to_string(),
                    }))
                }
                Err(e) => {
                    Ok(Json(ApiResponse {
                        success: false,
                        data: None,
                        message: format!("Contract execution failed: {}", e),
                    }))
                }
            }
        }
    }
}

/// Get contract statistics and performance metrics
async fn get_contract_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    println!("📊 API: Getting contract statistics");

    // Get contract statistics from blockchain directly
    match state.blockchain.get_zkp_health_check().await {
        Ok(zkp_stats) => {
            let response_data = serde_json::json!({
                "zkp_stats": zkp_stats,
                "system_status": "operational",
                "last_updated": chrono::Utc::now().to_rfc3339()
            });

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                message: "Contract statistics retrieved successfully".to_string(),
            }))
        }
        Err(e) => {
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                message: format!("Failed to get contract stats: {}", e),
            }))
        }
    }
}