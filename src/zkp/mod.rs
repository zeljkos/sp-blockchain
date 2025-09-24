pub mod settlement_proofs;
pub mod circuit;
pub mod trusted_setup;
pub mod circuits;
pub mod smart_contracts;

// Re-export main ZKP functionality
pub use settlement_proofs::{SettlementProofSystem, SettlementProof, ProofParameters};
pub use circuit::{SettlementCircuit, SettlementWitness};
pub use trusted_setup::*;
pub use circuits::*;
pub use smart_contracts::*;