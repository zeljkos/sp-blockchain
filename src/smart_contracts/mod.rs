pub mod settlement_contract;
pub mod vm;
pub mod contract_api;

// Re-export smart contract functionality
pub use settlement_contract::{SettlementContract, ContractState, ContractResult};
pub use vm::{SmartContractVM, ContractExecution};
pub use contract_api::{ContractAPI, ContractCall};