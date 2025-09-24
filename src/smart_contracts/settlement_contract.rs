use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::hash::Blake2bHash;
use crate::zkp::{SettlementProof, SettlementProofSystem};

/// Programmable smart contract for settlement rules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementContract {
    pub contract_id: String,
    pub version: u32,
    pub operators: Vec<String>,
    pub rules: SettlementRules,
    pub state: ContractState,
}

/// Settlement rules defined in the smart contract
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementRules {
    pub min_settlement_amount: u64,
    pub max_settlement_amount: u64,
    pub settlement_frequency_hours: u32,
    pub required_approvals: u32,
    pub dispute_timeout_hours: u32,
    pub fee_structure: FeeStructure,
    pub validation_rules: ValidationRules,
}

/// Fee structure for settlements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeStructure {
    pub base_fee_cents: u64,
    pub percentage_fee: f64, // 0.01 = 1%
    pub max_fee_cents: u64,
    pub operator_fee_split: HashMap<String, f64>, // Sum must equal 1.0
}

/// Validation rules for BCE records
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationRules {
    pub require_zkp_proof: bool,
    pub max_call_rate_cents: u64,
    pub max_data_rate_cents: u64,
    pub max_sms_rate_cents: u64,
    pub allowed_operators: Vec<String>,
    pub rate_validation_formula: String, // Simple formula string
}

/// Current state of the smart contract
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractState {
    pub total_processed: u64,
    pub last_settlement_time: u64,
    pub pending_settlements: Vec<String>,
    pub operator_balances: HashMap<String, i64>, // Can be negative (debt)
    pub dispute_cases: Vec<DisputeCase>,
    pub execution_count: u64,
}

/// Dispute case for settlement issues
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisputeCase {
    pub dispute_id: String,
    pub settlement_id: String,
    pub disputant: String,
    pub reason: String,
    pub evidence_hash: Blake2bHash,
    pub created_time: u64,
    pub status: DisputeStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Resolved,
    Rejected,
}

/// Result of contract execution
#[derive(Debug)]
pub struct ContractResult {
    pub success: bool,
    pub message: String,
    pub settlement_approved: bool,
    pub fees_calculated: u64,
    pub new_state: Option<ContractState>,
    pub required_actions: Vec<ContractAction>,
}

/// Actions required from contract execution
#[derive(Debug, Clone)]
pub enum ContractAction {
    TransferFunds { from: String, to: String, amount: u64 },
    CreateSettlement { settlement_id: String, amount: u64 },
    RequestApproval { from: String, settlement_id: String },
    RaiseDispute { settlement_id: String, reason: String },
    UpdateOperatorBalance { operator: String, change: i64 },
}

impl SettlementContract {
    /// Create a new settlement contract with default rules
    pub fn new(contract_id: String, operators: Vec<String>) -> Self {
        let operator_count = operators.len();
        let equal_split = 1.0 / operator_count as f64;

        let mut operator_fee_split = HashMap::new();
        let mut operator_balances = HashMap::new();

        for operator in &operators {
            operator_fee_split.insert(operator.clone(), equal_split);
            operator_balances.insert(operator.clone(), 0);
        }

        Self {
            contract_id,
            version: 1,
            operators: operators.clone(),
            rules: SettlementRules {
                min_settlement_amount: 1000, // 10.00 EUR
                max_settlement_amount: 1000000, // 10,000 EUR
                settlement_frequency_hours: 24,
                required_approvals: (operator_count as u32 + 1) / 2, // Majority
                dispute_timeout_hours: 72,
                fee_structure: FeeStructure {
                    base_fee_cents: 100, // 1.00 EUR
                    percentage_fee: 0.005, // 0.5%
                    max_fee_cents: 1000, // 10.00 EUR
                    operator_fee_split,
                },
                validation_rules: ValidationRules {
                    require_zkp_proof: true,
                    max_call_rate_cents: 50, // 0.50 EUR per minute
                    max_data_rate_cents: 10, // 0.10 EUR per MB
                    max_sms_rate_cents: 15, // 0.15 EUR per SMS
                    allowed_operators: operators,
                    rate_validation_formula: "call_minutes * call_rate + data_mb * data_rate + sms_count * sms_rate".to_string(),
                },
            },
            state: ContractState {
                total_processed: 0,
                last_settlement_time: 0,
                pending_settlements: Vec::new(),
                operator_balances,
                dispute_cases: Vec::new(),
                execution_count: 0,
            },
        }
    }

    /// Execute settlement validation through the smart contract
    pub fn execute_settlement(
        &mut self,
        settlement_id: String,
        total_amount: u64,
        operators_involved: Vec<String>,
        zkp_proof: Option<SettlementProof>,
        current_time: u64,
    ) -> ContractResult {
        println!("📋 Executing settlement contract for: {}", settlement_id);

        self.state.execution_count += 1;

        // Validation 1: Amount limits
        if total_amount < self.rules.min_settlement_amount {
            return ContractResult {
                success: false,
                message: format!("Amount {} below minimum {}", total_amount, self.rules.min_settlement_amount),
                settlement_approved: false,
                fees_calculated: 0,
                new_state: None,
                required_actions: vec![],
            };
        }

        if total_amount > self.rules.max_settlement_amount {
            return ContractResult {
                success: false,
                message: format!("Amount {} exceeds maximum {}", total_amount, self.rules.max_settlement_amount),
                settlement_approved: false,
                fees_calculated: 0,
                new_state: None,
                required_actions: vec![],
            };
        }

        // Validation 2: Settlement frequency
        if current_time < self.state.last_settlement_time + (self.rules.settlement_frequency_hours as u64 * 3600) {
            return ContractResult {
                success: false,
                message: "Settlement frequency limit exceeded".to_string(),
                settlement_approved: false,
                fees_calculated: 0,
                new_state: None,
                required_actions: vec![],
            };
        }

        // Validation 3: ZKP proof requirement
        if self.rules.validation_rules.require_zkp_proof && zkp_proof.is_none() {
            return ContractResult {
                success: false,
                message: "ZKP proof required but not provided".to_string(),
                settlement_approved: false,
                fees_calculated: 0,
                new_state: None,
                required_actions: vec![],
            };
        }

        // Validation 4: Verify ZKP proof if provided
        if let Some(proof) = zkp_proof {
            let zkp_system = SettlementProofSystem::new().expect("ZKP system initialization failed");
            match zkp_system.verify_proof(&proof) {
                Ok(true) => println!("✅ ZKP proof verification successful"),
                Ok(false) => {
                    return ContractResult {
                        success: false,
                        message: "ZKP proof verification failed".to_string(),
                        settlement_approved: false,
                        fees_calculated: 0,
                        new_state: None,
                        required_actions: vec![],
                    };
                }
                Err(e) => {
                    return ContractResult {
                        success: false,
                        message: format!("ZKP proof verification error: {}", e),
                        settlement_approved: false,
                        fees_calculated: 0,
                        new_state: None,
                        required_actions: vec![],
                    };
                }
            }
        }

        // Calculate fees
        let fees = self.calculate_fees(total_amount);

        // Create required actions
        let mut actions = vec![
            ContractAction::CreateSettlement {
                settlement_id: settlement_id.clone(),
                amount: total_amount,
            }
        ];

        // Request approvals from required operators
        let approvals_needed = self.rules.required_approvals;
        for (i, operator) in operators_involved.iter().enumerate() {
            if i < approvals_needed as usize {
                actions.push(ContractAction::RequestApproval {
                    from: operator.clone(),
                    settlement_id: settlement_id.clone(),
                });
            }
        }

        // Update contract state
        self.state.total_processed += total_amount;
        self.state.last_settlement_time = current_time;
        self.state.pending_settlements.push(settlement_id.clone());

        // Update operator balances (simplified)
        let per_operator_amount = total_amount / operators_involved.len() as u64;
        for operator in &operators_involved {
            if let Some(balance) = self.state.operator_balances.get_mut(operator) {
                *balance += per_operator_amount as i64;
            }

            actions.push(ContractAction::UpdateOperatorBalance {
                operator: operator.clone(),
                change: per_operator_amount as i64,
            });
        }

        println!("✅ Settlement contract execution successful");

        ContractResult {
            success: true,
            message: format!("Settlement {} approved with {} fees", settlement_id, fees),
            settlement_approved: true,
            fees_calculated: fees,
            new_state: Some(self.state.clone()),
            required_actions: actions,
        }
    }

    /// Calculate fees based on contract rules
    fn calculate_fees(&self, amount: u64) -> u64 {
        let base_fee = self.rules.fee_structure.base_fee_cents;
        let percentage_fee = (amount as f64 * self.rules.fee_structure.percentage_fee) as u64;
        let total_fee = base_fee + percentage_fee;

        total_fee.min(self.rules.fee_structure.max_fee_cents)
    }

    /// Validate BCE record rates according to contract rules
    pub fn validate_bce_rates(
        &self,
        call_rate: u64,
        data_rate: u64,
        sms_rate: u64,
    ) -> Result<(), String> {
        let rules = &self.rules.validation_rules;

        if call_rate > rules.max_call_rate_cents {
            return Err(format!("Call rate {} exceeds maximum {}", call_rate, rules.max_call_rate_cents));
        }

        if data_rate > rules.max_data_rate_cents {
            return Err(format!("Data rate {} exceeds maximum {}", data_rate, rules.max_data_rate_cents));
        }

        if sms_rate > rules.max_sms_rate_cents {
            return Err(format!("SMS rate {} exceeds maximum {}", sms_rate, rules.max_sms_rate_cents));
        }

        Ok(())
    }

    /// Create a dispute case
    pub fn create_dispute(
        &mut self,
        settlement_id: String,
        disputant: String,
        reason: String,
        evidence_hash: Blake2bHash,
        current_time: u64,
    ) -> String {
        let dispute_id = format!("dispute_{}_{}", settlement_id, current_time);

        let dispute = DisputeCase {
            dispute_id: dispute_id.clone(),
            settlement_id,
            disputant,
            reason,
            evidence_hash,
            created_time: current_time,
            status: DisputeStatus::Open,
        };

        self.state.dispute_cases.push(dispute);
        println!("⚖️  Created dispute case: {}", dispute_id);

        dispute_id
    }

    /// Get contract statistics
    pub fn get_stats(&self) -> ContractStats {
        ContractStats {
            contract_id: self.contract_id.clone(),
            version: self.version,
            total_executions: self.state.execution_count,
            total_processed: self.state.total_processed,
            pending_settlements: self.state.pending_settlements.len(),
            active_disputes: self.state.dispute_cases.iter().filter(|d| matches!(d.status, DisputeStatus::Open | DisputeStatus::UnderReview)).count(),
            operator_count: self.operators.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ContractStats {
    pub contract_id: String,
    pub version: u32,
    pub total_executions: u64,
    pub total_processed: u64,
    pub pending_settlements: usize,
    pub active_disputes: usize,
    pub operator_count: usize,
}

impl Default for SettlementContract {
    fn default() -> Self {
        Self::new(
            "default_settlement_contract".to_string(),
            vec![
                "tmobile-de".to_string(),
                "vodafone-uk".to_string(),
                "orange-fr".to_string(),
            ]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_contract_creation() {
        let operators = vec!["op1".to_string(), "op2".to_string()];
        let contract = SettlementContract::new("test_contract".to_string(), operators);

        assert_eq!(contract.operators.len(), 2);
        assert_eq!(contract.rules.required_approvals, 1); // (2+1)/2 = 1
        assert!(contract.state.operator_balances.contains_key("op1"));
    }

    #[test]
    fn test_settlement_execution() {
        let mut contract = SettlementContract::default();

        let result = contract.execute_settlement(
            "test_settlement".to_string(),
            5000, // 50.00 EUR
            vec!["tmobile-de".to_string(), "vodafone-uk".to_string()],
            None, // No ZKP proof for this test
            chrono::Utc::now().timestamp() as u64,
        );

        // Should fail because ZKP proof is required but not provided
        assert!(!result.success);
        assert!(result.message.contains("ZKP proof required"));
    }

    #[test]
    fn test_fee_calculation() {
        let contract = SettlementContract::default();
        let fees = contract.calculate_fees(10000); // 100.00 EUR

        // Base fee (1.00) + 0.5% of 100.00 = 1.00 + 0.50 = 1.50 EUR = 150 cents
        assert_eq!(fees, 150);
    }

    #[test]
    fn test_bce_rate_validation() {
        let contract = SettlementContract::default();

        // Valid rates
        assert!(contract.validate_bce_rates(30, 5, 10).is_ok());

        // Invalid call rate
        assert!(contract.validate_bce_rates(100, 5, 10).is_err());

        // Invalid data rate
        assert!(contract.validate_bce_rates(30, 50, 10).is_err());
    }
}