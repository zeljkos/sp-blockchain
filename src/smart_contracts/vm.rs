use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::*;

use crate::hash::Blake2bHash;
use crate::smart_contracts::settlement_contract::{SettlementContract, ContractResult, ContractAction};
use crate::zkp::SettlementProof;

/// Smart Contract Virtual Machine for executing settlement contracts
pub struct SmartContractVM {
    engine: Engine,
    contracts: HashMap<String, SettlementContract>,
    execution_limits: ExecutionLimits,
}

/// Execution limits for smart contracts
#[derive(Clone, Debug)]
pub struct ExecutionLimits {
    pub max_memory_pages: u32,
    pub max_execution_time_ms: u64,
    pub max_stack_size: u32,
}

/// Context for contract execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractExecution {
    pub contract_id: String,
    pub execution_id: String,
    pub caller: String,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub success: bool,
    pub return_data: Vec<u8>,
    pub events: Vec<ContractEvent>,
}

/// Events emitted during contract execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractEvent {
    pub event_type: String,
    pub data: HashMap<String, String>,
    pub timestamp: u64,
}

/// Contract call parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractCall {
    pub contract_id: String,
    pub method: String,
    pub parameters: HashMap<String, String>,
    pub caller: String,
    pub gas_limit: u64,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_memory_pages: 16,        // 1MB memory limit
            max_execution_time_ms: 5000, // 5 second timeout
            max_stack_size: 1024,        // 1KB stack
        }
    }
}

impl SmartContractVM {
    /// Create a new smart contract VM
    pub fn new() -> Result<Self, VMError> {
        let mut config = Config::new();
        config.wasm_multi_memory(true);
        config.wasm_memory64(false);

        let engine = Engine::new(&config)
            .map_err(|e| VMError::InitializationFailed(format!("Engine creation failed: {}", e)))?;

        Ok(Self {
            engine,
            contracts: HashMap::new(),
            execution_limits: ExecutionLimits::default(),
        })
    }

    /// Deploy a new settlement contract
    pub fn deploy_contract(
        &mut self,
        contract_id: String,
        operators: Vec<String>,
    ) -> Result<String, VMError> {
        println!("📜 Deploying settlement contract: {}", contract_id);

        let contract = SettlementContract::new(contract_id.clone(), operators);
        self.contracts.insert(contract_id.clone(), contract);

        println!("✅ Contract deployed successfully: {}", contract_id);
        Ok(contract_id)
    }

    /// Execute a contract method
    pub fn execute_contract(
        &mut self,
        call: ContractCall,
    ) -> Result<ContractExecution, VMError> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let execution_id = format!("exec_{}_{}", call.contract_id, start_time);

        println!("🔧 Executing contract method: {}.{}", call.contract_id, call.method);

        let mut execution = ContractExecution {
            contract_id: call.contract_id.clone(),
            execution_id: execution_id.clone(),
            caller: call.caller.clone(),
            timestamp: start_time,
            gas_limit: call.gas_limit,
            gas_used: 0,
            success: false,
            return_data: Vec::new(),
            events: Vec::new(),
        };

        // Get the contract
        let contract = self.contracts.get_mut(&call.contract_id)
            .ok_or_else(|| VMError::ContractNotFound(call.contract_id.clone()))?;

        // Execute the method based on the method name
        let result = match call.method.as_str() {
            "execute_settlement" => {
                self.execute_settlement_method(contract, &call, &mut execution)
            }
            "validate_bce_rates" => {
                self.validate_bce_rates_method(contract, &call, &mut execution)
            }
            "create_dispute" => {
                self.create_dispute_method(contract, &call, &mut execution)
            }
            "get_stats" => {
                self.get_stats_method(contract, &call, &mut execution)
            }
            _ => Err(VMError::MethodNotFound(call.method.clone())),
        };

        match result {
            Ok(return_data) => {
                execution.success = true;
                execution.return_data = return_data;
                execution.gas_used = 1000; // Simplified gas calculation

                execution.events.push(ContractEvent {
                    event_type: "execution_success".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("method".to_string(), call.method);
                        data.insert("gas_used".to_string(), execution.gas_used.to_string());
                        data
                    },
                    timestamp: start_time,
                });

                println!("✅ Contract execution successful: {}", execution_id);
            }
            Err(e) => {
                execution.success = false;
                execution.return_data = format!("Error: {}", e).into_bytes();

                execution.events.push(ContractEvent {
                    event_type: "execution_error".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("method".to_string(), call.method);
                        data.insert("error".to_string(), e.to_string());
                        data
                    },
                    timestamp: start_time,
                });

                println!("❌ Contract execution failed: {} - {}", execution_id, e);
            }
        }

        Ok(execution)
    }

    /// Execute settlement method
    fn execute_settlement_method(
        &self,
        contract: &mut SettlementContract,
        call: &ContractCall,
        execution: &mut ContractExecution,
    ) -> Result<Vec<u8>, VMError> {
        // Parse parameters
        let settlement_id = call.parameters.get("settlement_id")
            .ok_or(VMError::InvalidParameters("Missing settlement_id".to_string()))?;

        let total_amount: u64 = call.parameters.get("total_amount")
            .and_then(|s| s.parse().ok())
            .ok_or(VMError::InvalidParameters("Invalid total_amount".to_string()))?;

        let operators_str = call.parameters.get("operators")
            .ok_or(VMError::InvalidParameters("Missing operators".to_string()))?;
        let operators: Vec<String> = serde_json::from_str(operators_str)
            .map_err(|e| VMError::InvalidParameters(format!("Invalid operators format: {}", e)))?;

        // ZKP proof is optional for now
        let zkp_proof: Option<SettlementProof> = call.parameters.get("zkp_proof")
            .and_then(|s| serde_json::from_str(s).ok());

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Execute the settlement
        let result = contract.execute_settlement(
            settlement_id.clone(),
            total_amount,
            operators,
            zkp_proof,
            current_time,
        );

        // Add settlement event
        let event_data = {
            let mut data = HashMap::new();
            data.insert("settlement_id".to_string(), settlement_id.clone());
            data.insert("total_amount".to_string(), total_amount.to_string());
            data.insert("success".to_string(), result.success.to_string());
            data.insert("fees".to_string(), result.fees_calculated.to_string());
            data
        };

        execution.events.push(ContractEvent {
            event_type: "settlement_executed".to_string(),
            data: event_data,
            timestamp: current_time,
        });

        // Serialize result
        serde_json::to_vec(&result)
            .map_err(|e| VMError::SerializationFailed(format!("Result serialization failed: {}", e)))
    }

    /// Validate BCE rates method
    fn validate_bce_rates_method(
        &self,
        contract: &SettlementContract,
        call: &ContractCall,
        _execution: &mut ContractExecution,
    ) -> Result<Vec<u8>, VMError> {
        let call_rate: u64 = call.parameters.get("call_rate")
            .and_then(|s| s.parse().ok())
            .ok_or(VMError::InvalidParameters("Invalid call_rate".to_string()))?;

        let data_rate: u64 = call.parameters.get("data_rate")
            .and_then(|s| s.parse().ok())
            .ok_or(VMError::InvalidParameters("Invalid data_rate".to_string()))?;

        let sms_rate: u64 = call.parameters.get("sms_rate")
            .and_then(|s| s.parse().ok())
            .ok_or(VMError::InvalidParameters("Invalid sms_rate".to_string()))?;

        let result = contract.validate_bce_rates(call_rate, data_rate, sms_rate);

        let response = match result {
            Ok(()) => "valid".to_string(),
            Err(e) => format!("invalid: {}", e),
        };

        Ok(response.into_bytes())
    }

    /// Create dispute method
    fn create_dispute_method(
        &self,
        contract: &mut SettlementContract,
        call: &ContractCall,
        execution: &mut ContractExecution,
    ) -> Result<Vec<u8>, VMError> {
        let settlement_id = call.parameters.get("settlement_id")
            .ok_or(VMError::InvalidParameters("Missing settlement_id".to_string()))?;

        let reason = call.parameters.get("reason")
            .ok_or(VMError::InvalidParameters("Missing reason".to_string()))?;

        let evidence = call.parameters.get("evidence")
            .unwrap_or(&"".to_string());

        let evidence_hash = Blake2bHash::hash(evidence.as_bytes());
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let dispute_id = contract.create_dispute(
            settlement_id.clone(),
            call.caller.clone(),
            reason.clone(),
            evidence_hash,
            current_time,
        );

        // Add dispute event
        let event_data = {
            let mut data = HashMap::new();
            data.insert("dispute_id".to_string(), dispute_id.clone());
            data.insert("settlement_id".to_string(), settlement_id.clone());
            data.insert("disputant".to_string(), call.caller.clone());
            data.insert("reason".to_string(), reason.clone());
            data
        };

        execution.events.push(ContractEvent {
            event_type: "dispute_created".to_string(),
            data: event_data,
            timestamp: current_time,
        });

        Ok(dispute_id.into_bytes())
    }

    /// Get contract statistics
    fn get_stats_method(
        &self,
        contract: &SettlementContract,
        _call: &ContractCall,
        _execution: &mut ContractExecution,
    ) -> Result<Vec<u8>, VMError> {
        let stats = contract.get_stats();
        serde_json::to_vec(&stats)
            .map_err(|e| VMError::SerializationFailed(format!("Stats serialization failed: {}", e)))
    }

    /// Get contract by ID
    pub fn get_contract(&self, contract_id: &str) -> Option<&SettlementContract> {
        self.contracts.get(contract_id)
    }

    /// List all deployed contracts
    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }

    /// Get VM statistics
    pub fn get_vm_stats(&self) -> VMStats {
        VMStats {
            total_contracts: self.contracts.len(),
            total_executions: self.contracts.values()
                .map(|c| c.get_stats().total_executions)
                .sum(),
            memory_usage_kb: 0, // Simplified
        }
    }
}

impl Default for SmartContractVM {
    fn default() -> Self {
        Self::new().expect("VM initialization failed")
    }
}

#[derive(Debug, Serialize)]
pub struct VMStats {
    pub total_contracts: usize,
    pub total_executions: u64,
    pub memory_usage_kb: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum VMError {
    #[error("VM initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Execution timeout")]
    ExecutionTimeout,

    #[error("Out of gas")]
    OutOfGas,

    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_creation() {
        let vm = SmartContractVM::new().unwrap();
        assert_eq!(vm.contracts.len(), 0);
    }

    #[test]
    fn test_contract_deployment() {
        let mut vm = SmartContractVM::new().unwrap();
        let operators = vec!["tmobile-de".to_string(), "vodafone-uk".to_string()];

        let contract_id = vm.deploy_contract("test_contract".to_string(), operators).unwrap();
        assert_eq!(contract_id, "test_contract");
        assert!(vm.get_contract(&contract_id).is_some());
    }

    #[test]
    fn test_contract_execution() {
        let mut vm = SmartContractVM::new().unwrap();
        let operators = vec!["tmobile-de".to_string(), "vodafone-uk".to_string()];

        vm.deploy_contract("test_contract".to_string(), operators.clone()).unwrap();

        let mut call_params = HashMap::new();
        call_params.insert("call_rate".to_string(), "30".to_string());
        call_params.insert("data_rate".to_string(), "5".to_string());
        call_params.insert("sms_rate".to_string(), "10".to_string());

        let call = ContractCall {
            contract_id: "test_contract".to_string(),
            method: "validate_bce_rates".to_string(),
            parameters: call_params,
            caller: "test_caller".to_string(),
            gas_limit: 10000,
        };

        let execution = vm.execute_contract(call).unwrap();
        assert!(execution.success);
        assert_eq!(String::from_utf8(execution.return_data).unwrap(), "valid");
    }
}