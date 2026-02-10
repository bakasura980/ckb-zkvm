//! Public values committed by the guest program.
//!
//! These values are extracted from the proof and can be verified
//! without re-executing the transaction.

use serde::{Deserialize, Serialize};

use crate::H256;

/// Result for a single script execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptExecutionResult {
    /// Script hash that was executed (32 bytes).
    pub script_hash: H256,

    /// Script group type: 0 = Lock, 1 = Type.
    pub script_group_type: u8,

    /// Exit code from the CKB-VM execution.
    /// 0 indicates successful execution.
    pub exit_code: i64,

    /// Cycles consumed during this script's execution.
    pub cycles: u64,
}

impl ScriptExecutionResult {
    /// Create a new script execution result.
    pub fn new(script_hash: H256, script_group_type: u8, exit_code: i64, cycles: u64) -> Self {
        Self {
            script_hash,
            script_group_type,
            exit_code,
            cycles,
        }
    }

    /// Check if this script executed successfully.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if this is a lock script.
    pub fn is_lock(&self) -> bool {
        self.script_group_type == 0
    }

    /// Check if this is a type script.
    pub fn is_type(&self) -> bool {
        self.script_group_type == 1
    }

    /// Encode to bytes (49 bytes total).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(49);
        bytes.extend_from_slice(&self.script_hash);
        bytes.push(self.script_group_type);
        bytes.extend_from_slice(&self.exit_code.to_le_bytes());
        bytes.extend_from_slice(&self.cycles.to_le_bytes());
        bytes
    }

    /// Decode from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 49 {
            return None;
        }

        let mut script_hash = [0u8; 32];
        script_hash.copy_from_slice(&bytes[0..32]);
        let script_group_type = bytes[32];
        let exit_code = i64::from_le_bytes(bytes[33..41].try_into().ok()?);
        let cycles = u64::from_le_bytes(bytes[41..49].try_into().ok()?);

        Some(Self {
            script_hash,
            script_group_type,
            exit_code,
            cycles,
        })
    }
}

/// Public values committed by the guest during proof generation.
///
/// The guest commits these values using `sp1_zkvm::io::commit()`,
/// making them publicly verifiable outputs of the proof.
///
/// Supports both:
/// - Single-script mode (legacy): one script per proof
/// - Batched mode: all scripts in a transaction in one proof
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicValues {
    /// Transaction hash that was proven.
    pub tx_hash: H256,

    /// Exit code from the CKB-VM execution.
    /// For batched proofs, this is 0 only if ALL scripts succeeded.
    pub exit_code: i64,

    /// Total cycles consumed during execution.
    /// For batched proofs, this is the sum of all script cycles.
    pub cycles: u64,

    /// Individual script results (empty for single-script mode).
    pub script_results: Vec<ScriptExecutionResult>,
}

impl PublicValues {
    /// Create new public values (single-script mode).
    pub fn new(tx_hash: H256, exit_code: i64, cycles: u64) -> Self {
        Self {
            tx_hash,
            exit_code,
            cycles,
            script_results: Vec::new(),
        }
    }

    /// Create new public values for batched proof.
    pub fn new_batched(tx_hash: H256, script_results: Vec<ScriptExecutionResult>) -> Self {
        let all_success = script_results.iter().all(|r| r.is_success());
        let total_cycles: u64 = script_results.iter().map(|r| r.cycles).sum();

        Self {
            tx_hash,
            exit_code: if all_success { 0 } else { -1 },
            cycles: total_cycles,
            script_results,
        }
    }

    /// Check if execution was successful (exit_code == 0).
    /// For batched proofs, all scripts must have succeeded.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if this is a batched proof.
    pub fn is_batched(&self) -> bool {
        !self.script_results.is_empty()
    }

    /// Get the number of scripts proven.
    pub fn script_count(&self) -> usize {
        if self.script_results.is_empty() {
            1 // Single-script mode
        } else {
            self.script_results.len()
        }
    }

    /// Encode public values to bytes for SP1 commitment.
    ///
    /// Format:
    /// - tx_hash: 32 bytes
    /// - exit_code: 8 bytes (i64 LE)
    /// - cycles: 8 bytes (u64 LE)
    /// - script_count: 4 bytes (u32 LE)
    /// - script_results: N * 49 bytes each
    pub fn to_bytes(&self) -> Vec<u8> {
        let script_count = self.script_results.len() as u32;
        let mut bytes = Vec::with_capacity(52 + self.script_results.len() * 49);

        bytes.extend_from_slice(&self.tx_hash);
        bytes.extend_from_slice(&self.exit_code.to_le_bytes());
        bytes.extend_from_slice(&self.cycles.to_le_bytes());
        bytes.extend_from_slice(&script_count.to_le_bytes());

        for result in &self.script_results {
            bytes.extend_from_slice(&result.to_bytes());
        }

        bytes
    }

    /// Decode public values from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 52 {
            return None;
        }

        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&bytes[0..32]);

        let exit_code = i64::from_le_bytes(bytes[32..40].try_into().ok()?);
        let cycles = u64::from_le_bytes(bytes[40..48].try_into().ok()?);
        let script_count = u32::from_le_bytes(bytes[48..52].try_into().ok()?) as usize;

        // Validate length
        let expected_len = 52 + script_count * 49;
        if bytes.len() != expected_len {
            return None;
        }

        let mut script_results = Vec::with_capacity(script_count);
        for i in 0..script_count {
            let start = 52 + i * 49;
            let end = start + 49;
            let result = ScriptExecutionResult::from_bytes(&bytes[start..end])?;
            script_results.push(result);
        }

        Some(Self {
            tx_hash,
            exit_code,
            cycles,
            script_results,
        })
    }
}

impl Default for PublicValues {
    fn default() -> Self {
        Self {
            tx_hash: [0u8; 32],
            exit_code: 0,
            cycles: 0,
            script_results: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_values_roundtrip() {
        let pv = PublicValues::new([1u8; 32], 0, 1000);

        let bytes = pv.to_bytes();
        let decoded = PublicValues::from_bytes(&bytes).unwrap();

        assert_eq!(pv, decoded);
    }

    #[test]
    fn test_batched_public_values_roundtrip() {
        let script_results = vec![
            ScriptExecutionResult::new([1u8; 32], 0, 0, 500),
            ScriptExecutionResult::new([2u8; 32], 1, 0, 300),
        ];

        let pv = PublicValues::new_batched([0u8; 32], script_results);

        assert!(pv.is_batched());
        assert!(pv.is_success());
        assert_eq!(pv.script_count(), 2);
        assert_eq!(pv.cycles, 800);

        let bytes = pv.to_bytes();
        let decoded = PublicValues::from_bytes(&bytes).unwrap();

        assert_eq!(pv, decoded);
    }

    #[test]
    fn test_batched_failure() {
        let script_results = vec![
            ScriptExecutionResult::new([1u8; 32], 0, 0, 500),
            ScriptExecutionResult::new([2u8; 32], 1, -1, 300), // Failed script
        ];

        let pv = PublicValues::new_batched([0u8; 32], script_results);

        assert!(!pv.is_success());
        assert_eq!(pv.exit_code, -1);
    }

    #[test]
    fn test_is_success() {
        let success = PublicValues::new([0u8; 32], 0, 100);
        assert!(success.is_success());

        let failure = PublicValues::new([0u8; 32], -1, 100);
        assert!(!failure.is_success());
    }

    #[test]
    fn test_script_execution_result() {
        let result = ScriptExecutionResult::new([1u8; 32], 0, 0, 1000);
        assert!(result.is_success());
        assert!(result.is_lock());
        assert!(!result.is_type());

        let type_result = ScriptExecutionResult::new([2u8; 32], 1, 0, 500);
        assert!(type_result.is_type());
        assert!(!type_result.is_lock());
    }

    #[test]
    fn test_from_bytes_invalid_length() {
        let result = PublicValues::from_bytes(&[0u8; 10]);
        assert!(result.is_none());
    }
}
