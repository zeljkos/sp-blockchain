// Smart Contract Virtual Machine for 5-party SP consortium
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use log::{info, error};

use crate::hash::Blake2bHash;

#[derive(Error, Debug)]
pub enum VmError {
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Invalid instruction at position {0}")]
    InvalidInstruction(usize),
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("ZKP verification failed")]
    ZkpVerificationFailed,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Execution halted")]
    ExecutionHalted,
}

type Result<T> = std::result::Result<T, VmError>;

/// Instructions for the smart contract VM - Enhanced for 5-party consortium
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Instruction {
    // Stack operations
    Push(u64),
    Pop,
    Dup,
    Swap,
    
    // Arithmetic operations
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    
    // Comparison operations
    Eq,
    Lt,
    Gt,
    
    // Control flow
    Jump(usize),
    JumpIf(usize),
    Halt,
    
    // Storage operations
    Load(Blake2bHash),
    Store(Blake2bHash),
    
    // Cryptographic operations
    VerifyProof,
    CheckSignature,
    
    // Settlement-specific operations
    CalculateSettlement,
    GetTimestamp,
    
    // Logging
    Log(String),
    
    // 5-party consortium specific
    ValidateConsortiumMember(String),
    CheckMultiPartySignatures(u8), // Check N signatures
    CalculateMultilateralNetting,
}

/// Smart Contract Virtual Machine State
pub struct SmartContractVM {
    /// Execution stack
    stack: Vec<u64>,
    
    /// Program counter
    pc: usize,
    
    /// Contract storage
    storage: HashMap<Blake2bHash, u64>,
    
    /// Execution logs
    logs: Vec<String>,
    
    /// Contract bytecode
    bytecode: Vec<Instruction>,
    
    /// Gas limit (optional for future)
    gas_limit: Option<u64>,
    
    /// Gas used
    gas_used: u64,
    
    /// 5-party consortium members
    consortium_members: Vec<String>,
    
    /// Execution result
    result: Option<u64>,
    
    /// Halt flag
    halted: bool,
}

impl SmartContractVM {
    /// Create new VM instance for 5-party consortium
    pub fn new(bytecode: Vec<Instruction>) -> Self {
        let consortium_members = vec![
            "T-Mobile-DE".to_string(),
            "Vodafone-UK".to_string(),
            "Orange-FR".to_string(),
            "Telenor-NO".to_string(),
            "SFR-FR".to_string(),
        ];
        
        Self {
            stack: Vec::new(),
            pc: 0,
            storage: HashMap::new(),
            logs: Vec::new(),
            bytecode,
            gas_limit: Some(1_000_000), // 1M gas limit
            gas_used: 0,
            consortium_members,
            result: None,
            halted: false,
        }
    }
    
    /// Create VM with initial storage state
    pub fn with_storage(bytecode: Vec<Instruction>, initial_storage: HashMap<Blake2bHash, u64>) -> Self {
        let mut vm = Self::new(bytecode);
        vm.storage = initial_storage;
        vm
    }
    
    /// Execute the smart contract
    pub fn execute(&mut self) -> Result<u64> {
        info!("🚀 Starting smart contract execution for 5-party consortium");
        
        while !self.halted && self.pc < self.bytecode.len() {
            if let Some(gas_limit) = self.gas_limit {
                if self.gas_used >= gas_limit {
                    error!("Gas limit exceeded: {} >= {}", self.gas_used, gas_limit);
                    return Err(VmError::StorageError("Gas limit exceeded".to_string()));
                }
            }
            
            self.execute_instruction()?;
            self.gas_used += 1;
        }
        
        if self.halted {
            let result = self.result.unwrap_or(0);
            info!("✅ Smart contract execution completed. Result: {}", result);
            info!("⛽ Gas used: {}", self.gas_used);
            info!("📜 Execution logs: {} entries", self.logs.len());
            Ok(result)
        } else {
            error!("❌ Smart contract execution did not halt properly");
            Err(VmError::ExecutionHalted)
        }
    }
    
    /// Execute a single instruction
    fn execute_instruction(&mut self) -> Result<()> {
        let instruction = self.bytecode[self.pc].clone();
        
        match instruction {
            Instruction::Push(value) => {
                self.stack.push(value);
            }
            
            Instruction::Pop => {
                self.stack.pop().ok_or(VmError::StackUnderflow)?;
            }
            
            Instruction::Dup => {
                let value = *self.stack.last().ok_or(VmError::StackUnderflow)?;
                self.stack.push(value);
            }
            
            Instruction::Swap => {
                let len = self.stack.len();
                if len < 2 {
                    return Err(VmError::StackUnderflow);
                }
                self.stack.swap(len - 1, len - 2);
            }
            
            Instruction::Add => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(a.wrapping_add(b));
            }
            
            Instruction::Sub => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(a.wrapping_sub(b));
            }
            
            Instruction::Mul => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(a.wrapping_mul(b));
            }
            
            Instruction::Div => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(a / b);
            }
            
            Instruction::Mod => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(a % b);
            }
            
            Instruction::Eq => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(if a == b { 1 } else { 0 });
            }
            
            Instruction::Lt => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(if a < b { 1 } else { 0 });
            }
            
            Instruction::Gt => {
                let b = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let a = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.stack.push(if a > b { 1 } else { 0 });
            }
            
            Instruction::Jump(addr) => {
                if addr >= self.bytecode.len() {
                    return Err(VmError::InvalidInstruction(addr));
                }
                self.pc = addr;
                return Ok(()); // Don't increment PC
            }
            
            Instruction::JumpIf(addr) => {
                let condition = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                if condition != 0 {
                    if addr >= self.bytecode.len() {
                        return Err(VmError::InvalidInstruction(addr));
                    }
                    self.pc = addr;
                    return Ok(()); // Don't increment PC
                }
            }
            
            Instruction::Halt => {
                self.result = self.stack.last().copied();
                self.halted = true;
                return Ok(());
            }
            
            Instruction::Load(key) => {
                let value = self.storage.get(&key).copied().unwrap_or(0);
                self.stack.push(value);
            }
            
            Instruction::Store(key) => {
                let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                self.storage.insert(key, value);
            }
            
            Instruction::VerifyProof => {
                // Simulate ZKP verification - in real implementation, this would
                // verify actual ZK proofs using the trusted setup keys
                let _proof_data = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                
                // For demo purposes, always return success
                // In production, this would interface with the ZKP circuits
                self.stack.push(1); // Success
                info!("🔐 ZKP verification simulated (would verify real proof)");
            }
            
            Instruction::CheckSignature => {
                // Simulate signature verification
                let _signature = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                
                // For demo purposes, always return success
                // In production, this would verify ed25519 signatures
                self.stack.push(1); // Success
                info!("✅ Signature verification simulated (would verify real signature)");
            }
            
            Instruction::CalculateSettlement => {
                let exchange_rate = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let amount = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                
                // Calculate settlement with exchange rate (rate in basis points)
                let settlement_amount = (amount * exchange_rate) / 10000;
                self.stack.push(settlement_amount);
                info!("💰 Settlement calculated: {} -> {}", amount, settlement_amount);
            }
            
            Instruction::GetTimestamp => {
                let timestamp = chrono::Utc::now().timestamp() as u64;
                self.stack.push(timestamp);
            }
            
            Instruction::Log(message) => {
                info!("📝 Contract Log: {}", message);
                self.logs.push(message);
            }
            
            Instruction::ValidateConsortiumMember(member) => {
                let is_valid = self.consortium_members.contains(&member);
                self.stack.push(if is_valid { 1 } else { 0 });
                info!("👥 Consortium member validation: {} -> {}", member, is_valid);
            }
            
            Instruction::CheckMultiPartySignatures(required_count) => {
                // Check that we have at least required_count valid signatures on stack
                let mut valid_count = 0;
                
                for _ in 0..required_count {
                    if let Some(sig_valid) = self.stack.pop() {
                        if sig_valid == 1 {
                            valid_count += 1;
                        }
                    }
                }
                
                let success = valid_count >= required_count as u64;
                self.stack.push(if success { 1 } else { 0 });
                info!("✅ Multi-party signature check: {}/{} valid", valid_count, required_count);
            }
            
            Instruction::CalculateMultilateralNetting => {
                // Simplified 5-party netting calculation
                // In practice, this would load all bilateral amounts and calculate net positions
                let total_bilateral = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                
                // Simulate significant netting savings (typically 60-90% reduction)
                let netting_efficiency = 75; // 75% reduction
                let net_amount = (total_bilateral * (100 - netting_efficiency)) / 100;
                
                self.stack.push(net_amount);
                info!("🔄 5-party multilateral netting: {} -> {} ({}% reduction)", 
                      total_bilateral, net_amount, netting_efficiency);
            }
        }
        
        self.pc += 1;
        Ok(())
    }
    
    /// Get current stack state (for debugging)
    pub fn get_stack(&self) -> &[u64] {
        &self.stack
    }
    
    /// Get execution logs
    pub fn get_logs(&self) -> &[String] {
        &self.logs
    }
    
    /// Get storage state
    pub fn get_storage(&self) -> &HashMap<Blake2bHash, u64> {
        &self.storage
    }
    
    /// Get gas usage
    pub fn get_gas_used(&self) -> u64 {
        self.gas_used
    }
    
    /// Check if execution has halted
    pub fn is_halted(&self) -> bool {
        self.halted
    }
    
    /// Get execution result
    pub fn get_result(&self) -> Option<u64> {
        self.result
    }
}

/// Contract execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub caller: String,
    pub contract_address: Blake2bHash,
    pub block_number: u64,
    pub timestamp: u64,
    pub consortium_members: Vec<String>,
}

impl ExecutionContext {
    /// Create new execution context for 5-party consortium
    pub fn new_consortium_context(
        caller: String,
        contract_address: Blake2bHash,
        block_number: u64,
    ) -> Self {
        Self {
            caller,
            contract_address,
            block_number,
            timestamp: chrono::Utc::now().timestamp() as u64,
            consortium_members: vec![
                "T-Mobile-DE".to_string(),
                "Vodafone-UK".to_string(),
                "Orange-FR".to_string(),
                "Telenor-NO".to_string(),
                "SFR-FR".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_arithmetic() {
        let bytecode = vec![
            Instruction::Push(10),
            Instruction::Push(5),
            Instruction::Add,
            Instruction::Halt,
        ];
        
        let mut vm = SmartContractVM::new(bytecode);
        let result = vm.execute().unwrap();
        
        assert_eq!(result, 15);
        assert_eq!(vm.get_gas_used(), 4);
    }
    
    #[test]
    fn test_settlement_calculation() {
        let bytecode = vec![
            Instruction::Push(100000), // €1000 in cents
            Instruction::Push(11000),  // 1.10 exchange rate in basis points
            Instruction::CalculateSettlement,
            Instruction::Halt,
        ];
        
        let mut vm = SmartContractVM::new(bytecode);
        let result = vm.execute().unwrap();
        
        assert_eq!(result, 110000); // €1100 in cents
    }
    
    #[test]
    fn test_consortium_member_validation() {
        let bytecode = vec![
            Instruction::ValidateConsortiumMember("T-Mobile-DE".to_string()),
            Instruction::ValidateConsortiumMember("Invalid-Network".to_string()),
            Instruction::Add, // Should be 1 + 0 = 1
            Instruction::Halt,
        ];
        
        let mut vm = SmartContractVM::new(bytecode);
        let result = vm.execute().unwrap();
        
        assert_eq!(result, 1); // Only T-Mobile-DE is valid
    }
    
    #[test]
    fn test_multilateral_netting() {
        let bytecode = vec![
            Instruction::Push(1000000), // €10,000 total bilateral
            Instruction::CalculateMultilateralNetting,
            Instruction::Halt,
        ];
        
        let mut vm = SmartContractVM::new(bytecode);
        let result = vm.execute().unwrap();
        
        assert_eq!(result, 250000); // 75% reduction -> €2,500 net
    }
    
    #[test]
    fn test_control_flow() {
        let bytecode = vec![
            Instruction::Push(1),
            Instruction::JumpIf(4), // Jump to instruction 4 if true
            Instruction::Push(99),  // Should be skipped
            Instruction::Halt,
            Instruction::Push(42),  // Jump target
            Instruction::Halt,
        ];
        
        let mut vm = SmartContractVM::new(bytecode);
        let result = vm.execute().unwrap();
        
        assert_eq!(result, 42); // Should jump and push 42
    }
}