// Settlement smart contracts for 5-party SP consortium
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use log::info;

use crate::hash::Blake2bHash;
use super::vm::Instruction;

/// Smart contract compiler for 5-party consortium settlements
pub struct SettlementContractCompiler;

impl SettlementContractCompiler {
    /// Compile enhanced BCE batch validation contract for 5-party consortium
    pub fn compile_5party_bce_validator() -> Vec<Instruction> {
        vec![
            Instruction::Log("5-Party BCE Batch Validator Started".to_string()),

            // Load batch data from input
            Instruction::Push(0), // batch_id offset
            Instruction::Load(Blake2bHash::zero()),

            // Load encrypted BCE data
            Instruction::Push(1), // bce_data offset
            Instruction::Load(Blake2bHash::zero()),

            // Load privacy proof
            Instruction::Push(2), // proof offset
            Instruction::Load(Blake2bHash::zero()),

            // Verify privacy proof using 5-party circuit
            Instruction::VerifyProof,
            Instruction::JumpIf(25), // Jump to success if proof valid

            // Proof verification failed
            Instruction::Log("5-Party privacy proof verification failed".to_string()),
            Instruction::Push(0),
            Instruction::Halt,

            // Proof verification succeeded (address 25)
            Instruction::Log("5-Party privacy proof verified".to_string()),

            // Validate consortium member
            Instruction::Push(3), // home_network offset
            Instruction::Load(Blake2bHash::zero()),
            // Convert to string and validate (simplified for demo)
            Instruction::ValidateConsortiumMember("T-Mobile-DE".to_string()),
            
            Instruction::Push(4), // visited_network offset
            Instruction::Load(Blake2bHash::zero()),
            Instruction::ValidateConsortiumMember("Vodafone-UK".to_string()),

            // Both networks must be valid consortium members
            Instruction::Add,
            Instruction::Push(2),
            Instruction::Eq,
            Instruction::JumpIf(45), // Jump to signature check

            // Invalid consortium member
            Instruction::Log("Invalid consortium member".to_string()),
            Instruction::Push(0),
            Instruction::Halt,

            // Check network signatures (address 45)
            Instruction::Push(5), // home_network_sig offset
            Instruction::Load(Blake2bHash::zero()),
            Instruction::Push(6), // visited_network_sig offset
            Instruction::Load(Blake2bHash::zero()),

            // Verify both network signatures
            Instruction::CheckSignature,
            Instruction::Swap,
            Instruction::CheckSignature,
            Instruction::Add,
            Instruction::Push(2),
            Instruction::Eq,
            Instruction::JumpIf(60), // Jump to success

            // Signature verification failed
            Instruction::Log("Network signature verification failed".to_string()),
            Instruction::Push(0),
            Instruction::Halt,

            // All verifications passed (address 60)
            Instruction::Log("5-Party BCE batch validated successfully".to_string()),
            Instruction::Push(1),
            Instruction::Halt,
        ]
    }

    /// Compile 5-party multilateral netting contract
    pub fn compile_5party_netting_contract() -> Vec<Instruction> {
        vec![
            Instruction::Log("5-Party Multilateral Netting Started".to_string()),

            // Load all bilateral amounts for 5 parties (20 total amounts)
            // T-Mobile outgoing
            Instruction::Load(Blake2bHash::from_bytes([10; 32])), // T-Mobile->Vodafone
            Instruction::Load(Blake2bHash::from_bytes([11; 32])), // T-Mobile->Orange
            Instruction::Load(Blake2bHash::from_bytes([12; 32])), // T-Mobile->Telenor
            Instruction::Load(Blake2bHash::from_bytes([13; 32])), // T-Mobile->SFR

            // Vodafone outgoing
            Instruction::Load(Blake2bHash::from_bytes([14; 32])), // Vodafone->T-Mobile
            Instruction::Load(Blake2bHash::from_bytes([15; 32])), // Vodafone->Orange
            Instruction::Load(Blake2bHash::from_bytes([16; 32])), // Vodafone->Telenor
            Instruction::Load(Blake2bHash::from_bytes([17; 32])), // Vodafone->SFR

            // Orange outgoing
            Instruction::Load(Blake2bHash::from_bytes([18; 32])), // Orange->T-Mobile
            Instruction::Load(Blake2bHash::from_bytes([19; 32])), // Orange->Vodafone
            Instruction::Load(Blake2bHash::from_bytes([20; 32])), // Orange->Telenor
            Instruction::Load(Blake2bHash::from_bytes([21; 32])), // Orange->SFR

            // Telenor outgoing
            Instruction::Load(Blake2bHash::from_bytes([22; 32])), // Telenor->T-Mobile
            Instruction::Load(Blake2bHash::from_bytes([23; 32])), // Telenor->Vodafone
            Instruction::Load(Blake2bHash::from_bytes([24; 32])), // Telenor->Orange
            Instruction::Load(Blake2bHash::from_bytes([25; 32])), // Telenor->SFR

            // SFR outgoing
            Instruction::Load(Blake2bHash::from_bytes([26; 32])), // SFR->T-Mobile
            Instruction::Load(Blake2bHash::from_bytes([27; 32])), // SFR->Vodafone
            Instruction::Load(Blake2bHash::from_bytes([28; 32])), // SFR->Orange
            Instruction::Load(Blake2bHash::from_bytes([29; 32])), // SFR->Telenor

            // Calculate total bilateral amount (sum all 20 amounts)
            // This is simplified - in practice would be a loop
            Instruction::Add, Instruction::Add, Instruction::Add, Instruction::Add, // First 4
            Instruction::Add, Instruction::Add, Instruction::Add, Instruction::Add, // Next 4
            Instruction::Add, Instruction::Add, Instruction::Add, Instruction::Add, // Next 4
            Instruction::Add, Instruction::Add, Instruction::Add, Instruction::Add, // Next 4
            Instruction::Add, Instruction::Add, Instruction::Add, // Last 3 (19 adds total)

            // Apply 5-party multilateral netting
            Instruction::CalculateMultilateralNetting,

            // Store net result
            Instruction::Dup,
            Instruction::Store(Blake2bHash::from_bytes([30; 32])), // net_settlement_amount

            Instruction::Log("5-Party multilateral netting completed".to_string()),
            Instruction::Push(1),
            Instruction::Halt,
        ]
    }

    /// Compile 5-party settlement execution contract
    pub fn compile_5party_settlement_executor() -> Vec<Instruction> {
        vec![
            Instruction::Log("5-Party Settlement Executor Started".to_string()),

            // Load settlement proof
            Instruction::Push(0), // settlement_proof offset
            Instruction::Load(Blake2bHash::zero()),

            // Verify 5-party settlement calculation proof
            Instruction::VerifyProof,
            Instruction::JumpIf(15), // Jump if proof valid

            // Settlement proof invalid
            Instruction::Log("5-Party settlement proof verification failed".to_string()),
            Instruction::Push(0),
            Instruction::Halt,

            // Settlement proof valid (address 15)
            Instruction::Log("5-Party settlement proof verified".to_string()),

            // Load signatures from all 5 consortium members
            Instruction::Push(1), // T-Mobile signature
            Instruction::Load(Blake2bHash::zero()),
            Instruction::CheckSignature,
            
            Instruction::Push(2), // Vodafone signature
            Instruction::Load(Blake2bHash::zero()),
            Instruction::CheckSignature,
            
            Instruction::Push(3), // Orange signature
            Instruction::Load(Blake2bHash::zero()),
            Instruction::CheckSignature,
            
            Instruction::Push(4), // Telenor signature
            Instruction::Load(Blake2bHash::zero()),
            Instruction::CheckSignature,
            
            Instruction::Push(5), // SFR signature
            Instruction::Load(Blake2bHash::zero()),
            Instruction::CheckSignature,

            // Verify that at least 3 of 5 signatures are valid (consortium quorum)
            Instruction::CheckMultiPartySignatures(3),
            Instruction::JumpIf(45), // Jump to execution

            // Insufficient valid signatures
            Instruction::Log("Insufficient consortium signatures (need 3/5)".to_string()),
            Instruction::Push(0),
            Instruction::Halt,

            // Execute settlement (address 45)
            Instruction::Log("Executing 5-party settlement transfer".to_string()),

            // Load net settlement details
            Instruction::Load(Blake2bHash::from_bytes([30; 32])), // net_settlement_amount
            Instruction::Load(Blake2bHash::from_bytes([31; 32])), // creditor_count
            Instruction::Load(Blake2bHash::from_bytes([32; 32])), // debtor_count

            // Record execution timestamp
            Instruction::GetTimestamp,
            Instruction::Store(Blake2bHash::from_bytes([33; 32])), // execution_timestamp

            Instruction::Log("5-Party settlement executed successfully".to_string()),
            Instruction::Push(1),
            Instruction::Halt,
        ]
    }
}

/// Executable settlement contract for 5-party consortium
#[derive(Clone)]
pub struct ExecutableSettlementContract {
    pub contract_address: Blake2bHash,
    pub bytecode: Vec<Instruction>,
    pub state: HashMap<Blake2bHash, u64>,
    pub contract_type: ContractType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContractType {
    BceValidator,
    NettingCalculator,
    SettlementExecutor,
    CustomContract(String),
}

impl ExecutableSettlementContract {
    /// Create new 5-party BCE validation contract
    pub fn new_5party_bce_validator(contract_id: Blake2bHash) -> Self {
        Self {
            contract_address: contract_id,
            bytecode: SettlementContractCompiler::compile_5party_bce_validator(),
            state: HashMap::new(),
            contract_type: ContractType::BceValidator,
        }
    }

    /// Create new 5-party netting contract with initial bilateral amounts
    pub fn new_5party_netting_contract(
        contract_id: Blake2bHash,
        bilateral_amounts: &[(String, String, u64)], // (from, to, amount) for all 20 pairs
    ) -> Self {
        let mut state = HashMap::new();

        // Initialize all 20 bilateral amounts in storage
        // T-Mobile outgoing
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "T-Mobile-DE" && t == "Vodafone-UK") {
            state.insert(Blake2bHash::from_bytes([10; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "T-Mobile-DE" && t == "Orange-FR") {
            state.insert(Blake2bHash::from_bytes([11; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "T-Mobile-DE" && t == "Telenor-NO") {
            state.insert(Blake2bHash::from_bytes([12; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "T-Mobile-DE" && t == "SFR-FR") {
            state.insert(Blake2bHash::from_bytes([13; 32]), amount.2);
        }

        // Vodafone outgoing
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Vodafone-UK" && t == "T-Mobile-DE") {
            state.insert(Blake2bHash::from_bytes([14; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Vodafone-UK" && t == "Orange-FR") {
            state.insert(Blake2bHash::from_bytes([15; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Vodafone-UK" && t == "Telenor-NO") {
            state.insert(Blake2bHash::from_bytes([16; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Vodafone-UK" && t == "SFR-FR") {
            state.insert(Blake2bHash::from_bytes([17; 32]), amount.2);
        }

        // Orange outgoing
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Orange-FR" && t == "T-Mobile-DE") {
            state.insert(Blake2bHash::from_bytes([18; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Orange-FR" && t == "Vodafone-UK") {
            state.insert(Blake2bHash::from_bytes([19; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Orange-FR" && t == "Telenor-NO") {
            state.insert(Blake2bHash::from_bytes([20; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Orange-FR" && t == "SFR-FR") {
            state.insert(Blake2bHash::from_bytes([21; 32]), amount.2);
        }

        // Telenor outgoing
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Telenor-NO" && t == "T-Mobile-DE") {
            state.insert(Blake2bHash::from_bytes([22; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Telenor-NO" && t == "Vodafone-UK") {
            state.insert(Blake2bHash::from_bytes([23; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Telenor-NO" && t == "Orange-FR") {
            state.insert(Blake2bHash::from_bytes([24; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "Telenor-NO" && t == "SFR-FR") {
            state.insert(Blake2bHash::from_bytes([25; 32]), amount.2);
        }

        // SFR outgoing
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "SFR-FR" && t == "T-Mobile-DE") {
            state.insert(Blake2bHash::from_bytes([26; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "SFR-FR" && t == "Vodafone-UK") {
            state.insert(Blake2bHash::from_bytes([27; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "SFR-FR" && t == "Orange-FR") {
            state.insert(Blake2bHash::from_bytes([28; 32]), amount.2);
        }
        if let Some(amount) = bilateral_amounts.iter().find(|(f, t, _)| f == "SFR-FR" && t == "Telenor-NO") {
            state.insert(Blake2bHash::from_bytes([29; 32]), amount.2);
        }

        Self {
            contract_address: contract_id,
            bytecode: SettlementContractCompiler::compile_5party_netting_contract(),
            state,
            contract_type: ContractType::NettingCalculator,
        }
    }

    /// Create new 5-party settlement execution contract
    pub fn new_5party_settlement_executor(contract_id: Blake2bHash) -> Self {
        Self {
            contract_address: contract_id,
            bytecode: SettlementContractCompiler::compile_5party_settlement_executor(),
            state: HashMap::new(),
            contract_type: ContractType::SettlementExecutor,
        }
    }

    /// Get contract deployment data
    pub fn get_deployment_data(&self) -> (Blake2bHash, Vec<Instruction>) {
        (self.contract_address, self.bytecode.clone())
    }

    /// Get initial state for contract deployment
    pub fn get_initial_state(&self) -> &HashMap<Blake2bHash, u64> {
        &self.state
    }

    /// Get contract type
    pub fn get_contract_type(&self) -> &ContractType {
        &self.contract_type
    }
}

/// Contract factory for creating 5-party settlement workflows
pub struct FivePartySettlementFactory;

impl FivePartySettlementFactory {
    /// Create complete 5-party settlement workflow contracts
    pub fn create_complete_settlement_workflow(
        period_id: &str,
        bilateral_settlements: &[(String, String, u64)], // All 20 bilateral amounts
    ) -> Vec<ExecutableSettlementContract> {
        let mut contracts = Vec::new();

        // 1. BCE validation contract
        let bce_validator_addr = Blake2bHash::hash(
            format!("5party_bce_validator_{}", period_id).as_bytes()
        );
        contracts.push(ExecutableSettlementContract::new_5party_bce_validator(bce_validator_addr));

        // 2. 5-party netting contract
        let netting_addr = Blake2bHash::hash(
            format!("5party_netting_{}", period_id).as_bytes()
        );
        contracts.push(ExecutableSettlementContract::new_5party_netting_contract(
            netting_addr,
            bilateral_settlements,
        ));

        // 3. Settlement execution contract
        let executor_addr = Blake2bHash::hash(
            format!("5party_executor_{}", period_id).as_bytes()
        );
        contracts.push(ExecutableSettlementContract::new_5party_settlement_executor(executor_addr));

        info!("📄 Created complete 5-party settlement workflow: {} contracts", contracts.len());
        contracts
    }

    /// Create all consortium member pairs (20 total for 5 parties)
    pub fn get_all_consortium_pairs() -> Vec<(String, String)> {
        let members = vec![
            "T-Mobile-DE".to_string(),
            "Vodafone-UK".to_string(),
            "Orange-FR".to_string(),
            "Telenor-NO".to_string(),
            "SFR-FR".to_string(),
        ];

        let mut pairs = Vec::new();
        for from in &members {
            for to in &members {
                if from != to {
                    pairs.push((from.clone(), to.clone()));
                }
            }
        }

        pairs
    }

    /// Generate sample bilateral amounts for testing
    pub fn generate_sample_bilateral_amounts() -> Vec<(String, String, u64)> {
        vec![
            // T-Mobile outgoing
            ("T-Mobile-DE".to_string(), "Vodafone-UK".to_string(), 150000), // €1500
            ("T-Mobile-DE".to_string(), "Orange-FR".to_string(), 120000),    // €1200
            ("T-Mobile-DE".to_string(), "Telenor-NO".to_string(), 80000),     // €800
            ("T-Mobile-DE".to_string(), "SFR-FR".to_string(), 90000),        // €900

            // Vodafone outgoing
            ("Vodafone-UK".to_string(), "T-Mobile-DE".to_string(), 180000),  // €1800
            ("Vodafone-UK".to_string(), "Orange-FR".to_string(), 200000),    // €2000
            ("Vodafone-UK".to_string(), "Telenor-NO".to_string(), 110000),   // €1100
            ("Vodafone-UK".to_string(), "SFR-FR".to_string(), 75000),       // €750

            // Orange outgoing
            ("Orange-FR".to_string(), "T-Mobile-DE".to_string(), 95000),     // €950
            ("Orange-FR".to_string(), "Vodafone-UK".to_string(), 85000),     // €850
            ("Orange-FR".to_string(), "Telenor-NO".to_string(), 60000),      // €600
            ("Orange-FR".to_string(), "SFR-FR".to_string(), 70000),         // €700

            // Telenor outgoing
            ("Telenor-NO".to_string(), "T-Mobile-DE".to_string(), 125000),   // €1250
            ("Telenor-NO".to_string(), "Vodafone-UK".to_string(), 140000),   // €1400
            ("Telenor-NO".to_string(), "Orange-FR".to_string(), 55000),      // €550
            ("Telenor-NO".to_string(), "SFR-FR".to_string(), 45000),        // €450

            // SFR outgoing
            ("SFR-FR".to_string(), "T-Mobile-DE".to_string(), 100000),       // €1000
            ("SFR-FR".to_string(), "Vodafone-UK".to_string(), 65000),       // €650
            ("SFR-FR".to_string(), "Orange-FR".to_string(), 50000),         // €500
            ("SFR-FR".to_string(), "Telenor-NO".to_string(), 40000),        // €400
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_5party_bce_validator_compilation() {
        let bytecode = SettlementContractCompiler::compile_5party_bce_validator();
        assert!(!bytecode.is_empty());
        assert!(matches!(bytecode[0], Instruction::Log(_)));
        assert!(matches!(bytecode.last(), Some(Instruction::Halt)));
    }

    #[test]
    fn test_5party_netting_contract_creation() {
        let bilateral_amounts = FivePartySettlementFactory::generate_sample_bilateral_amounts();
        let contract_id = Blake2bHash::hash(b"test_netting");
        
        let contract = ExecutableSettlementContract::new_5party_netting_contract(
            contract_id, 
            &bilateral_amounts
        );

        assert_eq!(contract.contract_address, contract_id);
        assert!(!contract.bytecode.is_empty());
        assert_eq!(contract.state.len(), 20); // All 20 bilateral amounts
        assert!(matches!(contract.contract_type, ContractType::NettingCalculator));
    }

    #[test]
    fn test_complete_workflow_creation() {
        let bilateral_amounts = FivePartySettlementFactory::generate_sample_bilateral_amounts();
        let contracts = FivePartySettlementFactory::create_complete_settlement_workflow(
            "2024-Q1",
            &bilateral_amounts,
        );

        assert_eq!(contracts.len(), 3); // BCE validator + netting + executor
        
        // Verify contract types
        assert!(matches!(contracts[0].contract_type, ContractType::BceValidator));
        assert!(matches!(contracts[1].contract_type, ContractType::NettingCalculator));
        assert!(matches!(contracts[2].contract_type, ContractType::SettlementExecutor));
    }

    #[test]
    fn test_consortium_pairs_generation() {
        let pairs = FivePartySettlementFactory::get_all_consortium_pairs();
        assert_eq!(pairs.len(), 20); // 5 parties * 4 targets each = 20 pairs
        
        // Verify all members are represented
        let members = ["T-Mobile-DE", "Vodafone-UK", "Orange-FR", "Telenor-NO", "SFR-FR"];
        for member in &members {
            let outgoing_count = pairs.iter().filter(|(from, _)| from == member).count();
            assert_eq!(outgoing_count, 4); // Each member should have 4 outgoing relationships
        }
    }
}