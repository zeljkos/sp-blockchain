use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::smart_contracts::vm::{SmartContractVM, ContractCall, ContractExecution, VMError};
use crate::smart_contracts::settlement_contract::SettlementContract;
use crate::zkp::{SettlementProofSystem, SettlementProof};

/// API layer for smart contract interactions
pub struct ContractAPI {
    vm: Arc<Mutex<SmartContractVM>>,
    zkp_system: SettlementProofSystem,
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
    /// Create a new contract API
    pub fn new() -> Result<Self, ContractAPIError> {
        let vm = SmartContractVM::new()
            .map_err(|e| ContractAPIError::InitializationFailed(format!("VM init failed: {}", e)))?;

        let zkp_system = SettlementProofSystem::new()
            .map_err(|e| ContractAPIError::InitializationFailed(format!("ZKP init failed: {}", e)))?;

        Ok(Self {
            vm: Arc::new(Mutex::new(vm)),
            zkp_system,
        })
    }

    /// Deploy a default settlement contract
    pub fn deploy_default_settlement_contract(
        &self,
        contract_id: String,
        operators: Vec<String>,
    ) -> Result<DeploymentResponse, ContractAPIError> {
        println!("🚀 Deploying settlement contract via API: {}", contract_id);

        let mut vm = self.vm.lock().unwrap();

        match vm.deploy_contract(contract_id.clone(), operators) {
            Ok(deployed_id) => {
                let deployment_hash = crate::hash::Blake2bHash::hash(&deployed_id);

                Ok(DeploymentResponse {
                    contract_id: deployed_id,
                    success: true,
                    message: "Settlement contract deployed successfully".to_string(),
                    deployment_hash: hex::encode(deployment_hash.as_bytes()),
                })
            }
            Err(e) => Ok(DeploymentResponse {
                contract_id,
                success: false,
                message: format!("Deployment failed: {}", e),
                deployment_hash: "".to_string(),
            })
        }
    }

    /// Execute a settlement with optional ZKP generation
    pub fn execute_settlement(
        &self,
        contract_id: String,
        request: SettlementRequest,
    ) -> Result<ExecutionResponse, ContractAPIError> {
        println!("⚖️  Executing settlement via API: {}", request.settlement_id);

        // Generate ZKP proof if requested
        let zkp_proof = if request.generate_zkp {
            if let (Some(amounts), Some(rates)) = (&request.private_amounts, &request.private_rates) {
                match self.zkp_system.prove_bce_settlement(
                    request.total_amount_cents,
                    amounts.clone(),
                    rates.clone(),
                    &request.settlement_id,
                ) {
                    Ok(proof) => {
                        println!("🔐 ZKP proof generated for settlement");
                        Some(proof)
                    }
                    Err(e) => {
                        println!("⚠️  ZKP proof generation failed: {}", e);
                        None
                    }
                }
            } else {
                println!("⚠️  Cannot generate ZKP: missing private amounts or rates");
                None
            }
        } else {
            None
        };

        // Prepare contract call parameters
        let mut params = HashMap::new();
        params.insert("settlement_id".to_string(), request.settlement_id);
        params.insert("total_amount".to_string(), request.total_amount_cents.to_string());
        params.insert("operators".to_string(), serde_json::to_string(&request.operators).unwrap());

        if let Some(proof) = zkp_proof {
            params.insert("zkp_proof".to_string(), serde_json::to_string(&proof).unwrap());
        }

        let call = ContractCall {
            contract_id: contract_id.clone(),
            method: "execute_settlement".to_string(),
            parameters: params,
            caller: "api_caller".to_string(),
            gas_limit: 50000,
        };

        let mut vm = self.vm.lock().unwrap();
        match vm.execute_contract(call) {
            Ok(execution) => Ok(self.convert_execution_to_response(execution)),
            Err(e) => Err(ContractAPIError::ExecutionFailed(format!("Settlement execution failed: {}", e))),
        }
    }

    /// Validate BCE rates
    pub fn validate_bce_rates(
        &self,
        contract_id: String,
        request: RateValidationRequest,
    ) -> Result<ExecutionResponse, ContractAPIError> {
        println!("📊 Validating BCE rates via API");

        let mut params = HashMap::new();
        params.insert("call_rate".to_string(), request.call_rate_cents.to_string());
        params.insert("data_rate".to_string(), request.data_rate_cents.to_string());
        params.insert("sms_rate".to_string(), request.sms_rate_cents.to_string());

        let call = ContractCall {
            contract_id,
            method: "validate_bce_rates".to_string(),
            parameters: params,
            caller: "api_caller".to_string(),
            gas_limit: 10000,
        };

        let mut vm = self.vm.lock().unwrap();
        match vm.execute_contract(call) {
            Ok(execution) => Ok(self.convert_execution_to_response(execution)),
            Err(e) => Err(ContractAPIError::ExecutionFailed(format!("Rate validation failed: {}", e))),
        }
    }

    /// Create a dispute
    pub fn create_dispute(
        &self,
        contract_id: String,
        request: DisputeRequest,
    ) -> Result<ExecutionResponse, ContractAPIError> {
        println!("⚖️  Creating dispute via API: {}", request.settlement_id);

        let mut params = HashMap::new();
        params.insert("settlement_id".to_string(), request.settlement_id);
        params.insert("reason".to_string(), request.reason);
        params.insert("evidence".to_string(), request.evidence);

        let call = ContractCall {
            contract_id,
            method: "create_dispute".to_string(),
            parameters: params,
            caller: request.disputant,
            gas_limit: 20000,
        };

        let mut vm = self.vm.lock().unwrap();
        match vm.execute_contract(call) {
            Ok(execution) => Ok(self.convert_execution_to_response(execution)),
            Err(e) => Err(ContractAPIError::ExecutionFailed(format!("Dispute creation failed: {}", e))),
        }
    }

    /// Get contract statistics
    pub fn get_contract_stats(&self, contract_id: String) -> Result<ExecutionResponse, ContractAPIError> {
        let call = ContractCall {
            contract_id,
            method: "get_stats".to_string(),
            parameters: HashMap::new(),
            caller: "api_caller".to_string(),
            gas_limit: 5000,
        };

        let mut vm = self.vm.lock().unwrap();
        match vm.execute_contract(call) {
            Ok(execution) => Ok(self.convert_execution_to_response(execution)),
            Err(e) => Err(ContractAPIError::ExecutionFailed(format!("Stats retrieval failed: {}", e))),
        }
    }

    /// Verify a ZKP proof
    pub fn verify_zkp_proof(&self, proof: SettlementProof) -> Result<bool, ContractAPIError> {
        println!("🔍 Verifying ZKP proof via API");

        self.zkp_system.verify_proof(&proof)
            .map_err(|e| ContractAPIError::ZKPError(format!("Proof verification failed: {}", e)))
    }

    /// List all deployed contracts
    pub fn list_contracts(&self) -> Result<Vec<String>, ContractAPIError> {
        let vm = self.vm.lock().unwrap();
        Ok(vm.list_contracts())
    }

    /// Get contract details
    pub fn get_contract(&self, contract_id: &str) -> Result<Option<serde_json::Value>, ContractAPIError> {
        let vm = self.vm.lock().unwrap();
        if let Some(contract) = vm.get_contract(contract_id) {
            let stats = contract.get_stats();
            Ok(Some(serde_json::to_value(stats).unwrap()))
        } else {
            Ok(None)
        }
    }

    /// Batch process multiple settlements
    pub fn batch_execute_settlements(
        &self,
        contract_id: String,
        requests: Vec<SettlementRequest>,
    ) -> Result<Vec<ExecutionResponse>, ContractAPIError> {
        println!("📦 Batch executing {} settlements", requests.len());

        let mut responses = Vec::new();
        for request in requests {
            match self.execute_settlement(contract_id.clone(), request) {
                Ok(response) => responses.push(response),
                Err(e) => {
                    // Log error but continue with batch
                    println!("❌ Batch settlement failed: {}", e);
                    responses.push(ExecutionResponse {
                        execution_id: "failed".to_string(),
                        success: false,
                        result: format!("Error: {}", e),
                        gas_used: 0,
                        events: vec![],
                    });
                }
            }
        }

        Ok(responses)
    }

    /// Convert VM execution to API response
    fn convert_execution_to_response(&self, execution: ContractExecution) -> ExecutionResponse {
        let events: Vec<ContractEventResponse> = execution.events
            .into_iter()
            .map(|event| ContractEventResponse {
                event_type: event.event_type,
                data: event.data,
                timestamp: event.timestamp,
            })
            .collect();

        ExecutionResponse {
            execution_id: execution.execution_id,
            success: execution.success,
            result: String::from_utf8_lossy(&execution.return_data).to_string(),
            gas_used: execution.gas_used,
            events,
        }
    }

    /// Export ZKP verifying key for public verification
    pub fn export_zkp_verifying_key(&self) -> Result<Vec<u8>, ContractAPIError> {
        self.zkp_system.export_verifying_key()
            .map_err(|e| ContractAPIError::ZKPError(format!("VK export failed: {}", e)))
    }

    /// Get API statistics
    pub fn get_api_stats(&self) -> Result<APIStats, ContractAPIError> {
        let vm = self.vm.lock().unwrap();
        let vm_stats = vm.get_vm_stats();

        Ok(APIStats {
            total_contracts: vm_stats.total_contracts,
            total_executions: vm_stats.total_executions,
            memory_usage_kb: vm_stats.memory_usage_kb,
            zkp_system_ready: true,
        })
    }
}

impl Default for ContractAPI {
    fn default() -> Self {
        Self::new().expect("Contract API initialization failed")
    }
}

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

    #[test]
    fn test_contract_api_creation() {
        let api = ContractAPI::new().unwrap();
        let stats = api.get_api_stats().unwrap();
        assert_eq!(stats.total_contracts, 0);
        assert!(stats.zkp_system_ready);
    }

    #[test]
    fn test_contract_deployment() {
        let api = ContractAPI::new().unwrap();
        let operators = vec!["tmobile-de".to_string(), "vodafone-uk".to_string()];

        let response = api.deploy_default_settlement_contract(
            "test_contract".to_string(),
            operators,
        ).unwrap();

        assert!(response.success);
        assert_eq!(response.contract_id, "test_contract");
        assert!(!response.deployment_hash.is_empty());
    }

    #[test]
    fn test_bce_rate_validation() {
        let api = ContractAPI::new().unwrap();
        let operators = vec!["tmobile-de".to_string(), "vodafone-uk".to_string()];

        // Deploy contract first
        api.deploy_default_settlement_contract("test_contract".to_string(), operators).unwrap();

        // Test rate validation
        let request = RateValidationRequest {
            call_rate_cents: 30,
            data_rate_cents: 5,
            sms_rate_cents: 10,
        };

        let response = api.validate_bce_rates("test_contract".to_string(), request).unwrap();
        assert!(response.success);
        assert_eq!(response.result, "valid");
    }
}