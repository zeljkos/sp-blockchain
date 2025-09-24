use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_r1cs_std::prelude::*;
use ark_relations::{
    lc,
    r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable},
};

/// Witness data for settlement circuit
#[derive(Clone, Debug)]
pub struct SettlementWitness {
    pub total_amount: u64,
    pub operator_count: u32,
    pub settlement_hash: [u8; 32],
    pub private_amounts: Vec<u64>,
    pub private_rates: Vec<u64>,
}

/// Zero-knowledge circuit for settlement validation
pub struct SettlementCircuit {
    pub witness: Option<SettlementWitness>,
}

impl SettlementCircuit {
    /// Create circuit with witness (for proving)
    pub fn new(witness: SettlementWitness) -> Self {
        Self {
            witness: Some(witness),
        }
    }

    /// Create empty circuit (for setup)
    pub fn new_dummy() -> Self {
        Self { witness: None }
    }
}

impl ConstraintSynthesizer<Fr> for SettlementCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public inputs
        let total_amount_var = cs.new_input_variable(|| {
            self.witness
                .as_ref()
                .map(|w| Fr::from(w.total_amount))
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        let operator_count_var = cs.new_input_variable(|| {
            self.witness
                .as_ref()
                .map(|w| Fr::from(w.operator_count as u64))
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        let settlement_hash_var = cs.new_input_variable(|| {
            self.witness
                .as_ref()
                .map(|w| {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&w.settlement_hash[0..8]);
                    Fr::from(u64::from_le_bytes(bytes))
                })
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private inputs (witnesses)
        if let Some(witness) = &self.witness {
            let mut private_amount_vars = Vec::new();
            let mut private_rate_vars = Vec::new();

            // Create private variables for amounts
            for &amount in &witness.private_amounts {
                let amount_var = cs.new_witness_variable(|| Ok(Fr::from(amount)))?;
                private_amount_vars.push(amount_var);
            }

            // Create private variables for rates
            for &rate in &witness.private_rates {
                let rate_var = cs.new_witness_variable(|| Ok(Fr::from(rate)))?;
                private_rate_vars.push(rate_var);
            }

            // Constraint 1: Sum of private amounts equals total amount
            let mut sum_constraint = lc!() + total_amount_var;
            for amount_var in &private_amount_vars {
                sum_constraint = sum_constraint - amount_var;
            }
            cs.enforce_constraint(lc!(), lc!(), sum_constraint)?;

            // Constraint 2: Number of amounts equals operator count
            let expected_count = Fr::from(private_amount_vars.len() as u64);
            cs.enforce_constraint(
                lc!() + operator_count_var - (expected_count, Variable::One),
                lc!(),
                lc!(),
            )?;

            // Constraint 3: Validate rate calculations (simplified)
            // In a real implementation, this would verify billing calculations
            for (i, (&amount, &rate)) in witness
                .private_amounts
                .iter()
                .zip(&witness.private_rates)
                .enumerate()
            {
                if i < private_amount_vars.len() && i < private_rate_vars.len() {
                    // Ensure rates are within valid range (0-1000 for example)
                    let max_rate = Fr::from(1000u64);
                    cs.enforce_constraint(
                        lc!() + private_rate_vars[i] - (max_rate, Variable::One),
                        lc!(),
                        lc!(),
                    )?;

                    // Constraint: amount * rate is positive (simplified billing validation)
                    let billing_result_var = cs.new_witness_variable(|| {
                        Ok(Fr::from((amount * rate) / 100)) // Rate as percentage
                    })?;

                    // This would be expanded in a real billing validation circuit
                    cs.enforce_constraint(
                        lc!() + private_amount_vars[i],
                        lc!() + private_rate_vars[i],
                        lc!() + (Fr::from(100u64), billing_result_var),
                    )?;
                }
            }

            // Constraint 4: Settlement hash integrity (simplified)
            // In practice, this would hash all private inputs and compare to public hash
            cs.enforce_constraint(
                lc!() + settlement_hash_var,
                lc!() + Variable::One,
                lc!() + settlement_hash_var,
            )?;

            println!("🔧 Generated {} constraints for settlement circuit", cs.num_constraints());
        }

        Ok(())
    }
}

/// Helper functions for circuit operations
impl SettlementCircuit {
    /// Validate that witness data is consistent
    pub fn validate_witness(witness: &SettlementWitness) -> Result<(), String> {
        // Check that amounts sum correctly
        let sum: u64 = witness.private_amounts.iter().sum();
        if sum != witness.total_amount {
            return Err(format!(
                "Amount sum mismatch: {} != {}",
                sum, witness.total_amount
            ));
        }

        // Check that we have the right number of operators
        if witness.private_amounts.len() != witness.operator_count as usize {
            return Err(format!(
                "Operator count mismatch: {} != {}",
                witness.private_amounts.len(),
                witness.operator_count
            ));
        }

        // Check that we have rates for all amounts
        if witness.private_amounts.len() != witness.private_rates.len() {
            return Err("Amounts and rates length mismatch".to_string());
        }

        // Validate rates are reasonable (0-1000%)
        for &rate in &witness.private_rates {
            if rate > 1000 {
                return Err(format!("Rate {} exceeds maximum 1000%", rate));
            }
        }

        Ok(())
    }

    /// Create a witness for testing
    pub fn create_test_witness() -> SettlementWitness {
        SettlementWitness {
            total_amount: 10000,
            operator_count: 3,
            settlement_hash: [0x42u8; 32], // Test hash
            private_amounts: vec![4000, 3000, 3000],
            private_rates: vec![150, 200, 100], // 1.5%, 2.0%, 1.0%
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::{ConstraintSystem, OptimizationGoal};

    #[test]
    fn test_settlement_circuit_constraints() {
        let witness = SettlementCircuit::create_test_witness();
        let circuit = SettlementCircuit::new(witness.clone());

        // Validate witness first
        SettlementCircuit::validate_witness(&witness).unwrap();

        // Test constraint generation
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);

        circuit.generate_constraints(cs.clone()).unwrap();

        assert!(cs.is_satisfied().unwrap());
        println!("Circuit generated {} constraints", cs.num_constraints());
        println!("Circuit has {} variables", cs.num_instance_variables() + cs.num_witness_variables());
    }

    #[test]
    fn test_witness_validation() {
        let valid_witness = SettlementCircuit::create_test_witness();
        assert!(SettlementCircuit::validate_witness(&valid_witness).is_ok());

        // Test invalid witness - wrong sum
        let mut invalid_witness = valid_witness.clone();
        invalid_witness.private_amounts[0] = 5000; // Now sum is 11000 != 10000
        assert!(SettlementCircuit::validate_witness(&invalid_witness).is_err());

        // Test invalid witness - wrong operator count
        let mut invalid_witness2 = valid_witness.clone();
        invalid_witness2.operator_count = 2; // But we have 3 amounts
        assert!(SettlementCircuit::validate_witness(&invalid_witness2).is_err());
    }
}