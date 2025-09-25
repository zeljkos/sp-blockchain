pub mod settlement_contract;
pub mod contract_api;

// Re-export smart contract functionality using ZKP-enabled systems
pub use settlement_contract::{SettlementContract, ContractState, ContractResult};
pub use contract_api::ContractAPI;