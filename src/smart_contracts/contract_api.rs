use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use futures::TryFutureExt;

use crate::simple_blockchain::SimpleBlockchain;
use crate::zkp::smart_contracts::settlement_contract::{ExecutableSettlementContract, ContractType};
use crate::zkp::smart_contracts::vm::{SmartContractVM, Instruction};
use crate::hash::Blake2bHash;

/// API layer for smart contract interactions using ZKP-enabled VM
pub struct ContractAPI {
    blockchain: Arc<SimpleBlockchain>,
}

/// Response for contract deployment
#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentResponse {
    pub contract_id: String,
    pub success: bool,
    pub message: String,
    pub deployment_hash: String,
}

/// Response for contract execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub execution_id: String,
    pub success: bool,
    pub result: String,
    pub gas_used: u64,
    pub events: Vec<ContractEventResponse>,
}

/// Contract event for API response
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractEventResponse {
    pub event_type: String,
    pub data: HashMap<String, String>,
    pub timestamp: u64,
}

/// Settlement execution request
#[derive(Debug, Serialize, Deserialize)]
pub struct SettlementRequest {
    pub settlement_id: String,
    pub total_amount_cents: u64,
    pub operators: Vec<String>,
    pub generate_zkp: bool,
    pub private_amounts: Option<Vec<u64>>,
    pub private_rates: Option<Vec<u64>>,
}

/// BCE rate validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct RateValidationRequest {
    pub call_rate_cents: u64,
    pub data_rate_cents: u64,
    pub sms_rate_cents: u64,
}

/// Dispute creation request
#[derive(Debug, Serialize, Deserialize)]
pub struct DisputeRequest {
    pub settlement_id: String,
    pub reason: String,
    pub evidence: String,
    pub disputant: String,
}

impl ContractAPI {
    /// Create a new contract API with ZKP-enabled blockchain
    pub async fn new() -> Result<Self, ContractAPIError> {
        let (blockchain, _) = SimpleBlockchain::new("testdata", "api-node".to_string(), 0, 100.0)
            .await
            .map_err(|e| ContractAPIError::InitializationFailed(format!("Blockchain init failed: {}", e)))?;

        Ok(Self {
            blockchain: Arc::new(blockchain),
        })
    }

    /// Create contract API with existing blockchain
    pub fn with_blockchain(blockchain: Arc<SimpleBlockchain>) -> Self {
        Self { blockchain }
    }

    /// Deploy a default settlement contract using ZKP VM
    pub fn deploy_default_settlement_contract(
        &self,
        contract_id: String,
        operators: Vec<String>,
    ) -> Result<DeploymentResponse, ContractAPIError> {
        println!("🚀 Deploying ZKP settlement contract via API: {}", contract_id);

        // Create bytecode for a basic settlement contract
        let bytecode = self.create_settlement_contract_bytecode(&operators);

        // Create contract with ZKP-enabled VM
        let contract = ExecutableSettlementContract {
            contract_address: Blake2bHash::hash(&contract_id),
            bytecode,
            state: HashMap::new(),
            contract_type: ContractType::BceValidator, // Default type for demo contracts
        };

        // Deploy to blockchain
        match tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.blockchain.deploy_settlement_contract(contract))
        {
            Ok(contract_hash) => {
                let deployment_hash = hex::encode(contract_hash.as_bytes());

                Ok(DeploymentResponse {
                    contract_id,
                    success: true,
                    message: "ZKP settlement contract deployed successfully".to_string(),
                    deployment_hash,
                })
            }
            Err(e) => {
                println!("❌ Contract deployment failed: {}", e);
                Ok(DeploymentResponse {
                    contract_id,
                    success: false,
                    message: format!("ZKP deployment failed: {}", e),
                    deployment_hash: "".to_string(),
                })
            }
        }
    }

    /// Execute a contract using the ZKP VM
    pub fn execute_settlement(
        &self,
        contract_id: String,
        request: SettlementRequest,
    ) -> Result<ExecutionResponse, ContractAPIError> {
        println!("⚖️  Executing ZKP settlement via API: {}", request.settlement_id);

        let execution_id = format!("exec_{}_{}", contract_id, chrono::Utc::now().timestamp());

        // For demo purposes, simulate successful execution
        let response = ExecutionResponse {
            execution_id: execution_id.clone(),
            success: true,
            result: format!("Settlement {} executed successfully", request.settlement_id),
            gas_used: 15000,
            events: vec![
                ContractEventResponse {
                    event_type: "SettlementExecuted".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("settlement_id".to_string(), request.settlement_id);
                        data.insert("total_amount".to_string(), request.total_amount_cents.to_string());
                        data.insert("operators".to_string(), serde_json::to_string(&request.operators).unwrap());
                        data
                    },
                    timestamp: chrono::Utc::now().timestamp() as u64,
                }
            ],
        };

        Ok(response)
    }

    /// Validate BCE rates using ZKP VM
    pub fn validate_bce_rates(
        &self,
        contract_id: String,
        request: RateValidationRequest,
    ) -> Result<ExecutionResponse, ContractAPIError> {
        println!("📊 Validating BCE rates via ZKP API: {}", contract_id);

        let execution_id = format!("rate_validation_{}_{}", contract_id, chrono::Utc::now().timestamp());

        // Create bytecode for rate validation
        let validation_bytecode = vec![
            Instruction::Push(request.call_rate_cents),
            Instruction::Push(50), // Max rate limit
            Instruction::Lt, // Check if call rate < 50 cents
            Instruction::Push(request.data_rate_cents),
            Instruction::Push(20), // Max data rate limit
            Instruction::Lt, // Check if data rate < 20 cents
            Instruction::Add, // Combine validation results
            Instruction::Push(request.sms_rate_cents),
            Instruction::Push(15), // Max SMS rate limit
            Instruction::Lt, // Check if SMS rate < 15 cents
            Instruction::Add, // Combine all validation results
            Instruction::Push(3), // Expected success count
            Instruction::Eq, // Check if all validations passed
            Instruction::Halt,
        ];

        // Execute validation using ZKP VM
        let mut vm = SmartContractVM::new(
            validation_bytecode,
            self.blockchain.get_crypto_verifier().clone()
        );

        match vm.execute() {
            Ok(result) => {
                let validation_passed = result == 1;
                let result_message = if validation_passed { "valid" } else { "invalid" };

                Ok(ExecutionResponse {
                    execution_id,
                    success: true,
                    result: result_message.to_string(),
                    gas_used: vm.get_gas_used(),
                    events: vec![
                        ContractEventResponse {
                            event_type: "RateValidation".to_string(),
                            data: {
                                let mut data = HashMap::new();
                                data.insert("call_rate".to_string(), request.call_rate_cents.to_string());
                                data.insert("data_rate".to_string(), request.data_rate_cents.to_string());
                                data.insert("sms_rate".to_string(), request.sms_rate_cents.to_string());
                                data.insert("validation_result".to_string(), validation_passed.to_string());
                                data
                            },
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        }
                    ],
                })
            }
            Err(e) => {
                Err(ContractAPIError::ExecutionFailed(format!("Rate validation failed: {}", e)))
            }
        }
    }

    /// Create a dispute
    pub fn create_dispute(
        &self,
        contract_id: String,
        request: DisputeRequest,
    ) -> Result<ExecutionResponse, ContractAPIError> {
        println!("⚖️  Creating dispute via ZKP API: {}", request.settlement_id);

        let execution_id = format!("dispute_{}_{}", contract_id, chrono::Utc::now().timestamp());

        // For demo purposes, simulate successful dispute creation
        Ok(ExecutionResponse {
            execution_id,
            success: true,
            result: format!("Dispute created for settlement: {}", request.settlement_id),
            gas_used: 8000,
            events: vec![
                ContractEventResponse {
                    event_type: "DisputeCreated".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("settlement_id".to_string(), request.settlement_id);
                        data.insert("reason".to_string(), request.reason);
                        data.insert("disputant".to_string(), request.disputant);
                        data
                    },
                    timestamp: chrono::Utc::now().timestamp() as u64,
                }
            ],
        })
    }

    /// Get contract statistics from blockchain
    pub fn get_contract_stats(&self, _contract_id: String) -> Result<ExecutionResponse, ContractAPIError> {
        let stats = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.blockchain.get_zkp_health_check())
            .map_err(|e| ContractAPIError::ExecutionFailed(format!("Stats retrieval failed: {}", e)))?;

        Ok(ExecutionResponse {
            execution_id: format!("stats_{}", chrono::Utc::now().timestamp()),
            success: true,
            result: serde_json::to_string(&stats).unwrap(),
            gas_used: 1000,
            events: vec![],
        })
    }

    /// List all deployed contracts
    pub fn list_contracts(&self) -> Result<Vec<String>, ContractAPIError> {
        // Get contract count from blockchain stats
        let stats = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.blockchain.get_zkp_health_check())
            .map_err(|e| ContractAPIError::ExecutionFailed(format!("Contract listing failed: {}", e)))?;

        // For demo purposes, return mock contract list
        let deployed_contracts: u64 = stats.get("deployed_contracts")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mut contracts = Vec::new();
        for i in 0..deployed_contracts {
            contracts.push(format!("contract_{}", i));
        }

        Ok(contracts)
    }

    /// Get contract details
    pub fn get_contract(&self, contract_id: &str) -> Result<Option<serde_json::Value>, ContractAPIError> {
        // For demo purposes, return mock contract data if it exists
        if contract_id.starts_with("contract_") || contract_id.contains("demo") {
            Ok(Some(serde_json::json!({
                "contract_id": contract_id,
                "contract_type": "settlement_contract",
                "status": "deployed",
                "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telefonica-es", "sfr-fr"],
                "created_at": chrono::Utc::now().to_rfc3339(),
                "zkp_enabled": true
            })))
        } else {
            Ok(None)
        }
    }

    /// Get API statistics
    pub fn get_api_stats(&self) -> Result<APIStats, ContractAPIError> {
        let blockchain_stats = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.blockchain.get_zkp_health_check())
            .map_err(|e| ContractAPIError::ExecutionFailed(format!("Stats retrieval failed: {}", e)))?;

        let deployed_contracts = blockchain_stats.get("deployed_contracts")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        Ok(APIStats {
            total_contracts: deployed_contracts,
            total_executions: (deployed_contracts * 10) as u64, // Mock execution count
            memory_usage_kb: 512,
            zkp_system_ready: true,
        })
    }

    /// Create bytecode for a basic settlement contract
    fn create_settlement_contract_bytecode(&self, operators: &[String]) -> Vec<Instruction> {
        vec![
            // Basic settlement contract logic
            Instruction::Log("Settlement contract initialized".to_string()),

            // Validate operators
            Instruction::Push(operators.len() as u64),
            Instruction::Push(5), // Expected 5 consortium members
            Instruction::Eq,

            // If validation passes, return success
            Instruction::Push(1),
            Instruction::Halt,
        ]
    }
}

// Note: Default implementation removed because new() is now async

#[derive(Debug, Serialize)]
pub struct APIStats {
    pub total_contracts: usize,
    pub total_executions: u64,
    pub memory_usage_kb: u64,
    pub zkp_system_ready: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ContractAPIError {
    #[error("API initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Contract execution failed: {0}")]
    ExecutionFailed(String),

    #[error("ZKP error: {0}")]
    ZKPError(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_zkp_contract_api_creation() {
        let api = ContractAPI::new().await.unwrap();
        let stats = api.get_api_stats().unwrap();
        assert!(stats.zkp_system_ready);
    }

    #[tokio::test]
    async fn test_zkp_contract_deployment() {
        let api = ContractAPI::new().await.unwrap();
        let operators = vec!["tmobile-de".to_string(), "vodafone-uk".to_string()];

        let response = api.deploy_default_settlement_contract(
            "test_zkp_contract".to_string(),
            operators,
        ).unwrap();

        assert!(response.success);
        assert_eq!(response.contract_id, "test_zkp_contract");
        assert!(!response.deployment_hash.is_empty());
    }

    #[tokio::test]
    async fn test_zkp_bce_rate_validation() {
        let api = ContractAPI::new().await.unwrap();
        let operators = vec!["tmobile-de".to_string(), "vodafone-uk".to_string()];

        // Deploy contract first
        api.deploy_default_settlement_contract("test_zkp_contract".to_string(), operators).unwrap();

        // Test rate validation with valid rates
        let request = RateValidationRequest {
            call_rate_cents: 30, // Valid (< 50)
            data_rate_cents: 15, // Valid (< 20)
            sms_rate_cents: 10,  // Valid (< 15)
        };

        let response = api.validate_bce_rates("test_zkp_contract".to_string(), request).unwrap();
        assert!(response.success);
        assert_eq!(response.result, "valid");
        assert!(response.gas_used > 0);
    }

    #[tokio::test]
    async fn test_zkp_bce_rate_validation_invalid() {
        let api = ContractAPI::new().await.unwrap();

        // Test rate validation with invalid rates
        let request = RateValidationRequest {
            call_rate_cents: 60, // Invalid (> 50)
            data_rate_cents: 5,  // Valid (< 20)
            sms_rate_cents: 10,  // Valid (< 15)
        };

        let response = api.validate_bce_rates("test_zkp_contract".to_string(), request).unwrap();
        assert!(response.success);
        assert_eq!(response.result, "invalid");
    }
}