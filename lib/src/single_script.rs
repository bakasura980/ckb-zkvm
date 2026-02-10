//! Public values for single-script parallel proving.
//!
//! Defines the per-script public values format used by the parallel guest
//! program (`program-64-parallel`) and decoded by the aggregation guest
//! (`program-64-aggregation`) and the host orchestrator.

use serde::{Deserialize, Serialize};

/// Public values committed by a single script group proof.
///
/// Each per-script SP1 proof commits exactly these values, allowing the
/// aggregation guest to verify what was proven and reconstruct the
/// batched `PublicValues` output.
///
/// ## Binary Format (81 bytes, little-endian)
///
/// | Offset | Size | Field |
/// |--------|------|-------|
/// | 0      | 32   | tx_hash |
/// | 32     | 32   | script_hash |
/// | 64     | 1    | script_group_type (0=Lock, 1=Type) |
/// | 65     | 8    | exit_code (i64 LE) |
/// | 73     | 8    | cycles (u64 LE) |
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleScriptPublicValues {
    /// Transaction hash that was proven.
    pub tx_hash: [u8; 32],

    /// Script hash that was executed.
    pub script_hash: [u8; 32],

    /// Script group type: 0 = Lock, 1 = Type.
    pub script_group_type: u8,

    /// Exit code from CKB-VM execution. 0 = success.
    pub exit_code: i64,

    /// Cycles consumed during execution.
    pub cycles: u64,
}

/// Encoded size of `SingleScriptPublicValues` in bytes.
pub const SINGLE_SCRIPT_PV_SIZE: usize = 81;

impl SingleScriptPublicValues {
    /// Create new single script public values.
    pub fn new(
        tx_hash: [u8; 32],
        script_hash: [u8; 32],
        script_group_type: u8,
        exit_code: i64,
        cycles: u64,
    ) -> Self {
        Self {
            tx_hash,
            script_hash,
            script_group_type,
            exit_code,
            cycles,
        }
    }

    /// Check if execution was successful.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Encode to bytes (81 bytes, little-endian).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(SINGLE_SCRIPT_PV_SIZE);
        bytes.extend_from_slice(&self.tx_hash);
        bytes.extend_from_slice(&self.script_hash);
        bytes.push(self.script_group_type);
        bytes.extend_from_slice(&self.exit_code.to_le_bytes());
        bytes.extend_from_slice(&self.cycles.to_le_bytes());
        bytes
    }

    /// Decode from bytes. Returns `None` if the input is not exactly 81 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != SINGLE_SCRIPT_PV_SIZE {
            return None;
        }

        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&bytes[0..32]);

        let mut script_hash = [0u8; 32];
        script_hash.copy_from_slice(&bytes[32..64]);

        let script_group_type = bytes[64];

        let exit_code = i64::from_le_bytes(
            bytes[65..73].try_into().ok()?,
        );
        let cycles = u64::from_le_bytes(
            bytes[73..81].try_into().ok()?,
        );

        Some(Self {
            tx_hash,
            script_hash,
            script_group_type,
            exit_code,
            cycles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let pv = SingleScriptPublicValues::new(
            [1u8; 32],
            [2u8; 32],
            0, // Lock
            0,
            1000,
        );

        let bytes = pv.to_bytes();
        assert_eq!(bytes.len(), SINGLE_SCRIPT_PV_SIZE);

        let decoded = SingleScriptPublicValues::from_bytes(&bytes).unwrap();
        assert_eq!(pv, decoded);
    }

    #[test]
    fn test_type_script() {
        let pv = SingleScriptPublicValues::new(
            [0xAA; 32],
            [0xBB; 32],
            1, // Type
            0,
            500,
        );

        let bytes = pv.to_bytes();
        let decoded = SingleScriptPublicValues::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.script_group_type, 1);
        assert!(decoded.is_success());
    }

    #[test]
    fn test_failure() {
        let pv = SingleScriptPublicValues::new(
            [0u8; 32],
            [0u8; 32],
            0,
            -1,
            100,
        );

        assert!(!pv.is_success());

        let bytes = pv.to_bytes();
        let decoded = SingleScriptPublicValues::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.exit_code, -1);
    }

    #[test]
    fn test_invalid_length() {
        assert!(SingleScriptPublicValues::from_bytes(&[0u8; 10]).is_none());
        assert!(SingleScriptPublicValues::from_bytes(&[0u8; 80]).is_none());
        assert!(SingleScriptPublicValues::from_bytes(&[0u8; 82]).is_none());
    }

    #[test]
    fn test_exact_byte_layout() {
        let pv = SingleScriptPublicValues::new(
            [0x01; 32],
            [0x02; 32],
            1,
            42,
            12345,
        );

        let bytes = pv.to_bytes();

        // tx_hash at offset 0
        assert_eq!(&bytes[0..32], &[0x01; 32]);
        // script_hash at offset 32
        assert_eq!(&bytes[32..64], &[0x02; 32]);
        // script_group_type at offset 64
        assert_eq!(bytes[64], 1);
        // exit_code at offset 65 (i64 LE)
        assert_eq!(&bytes[65..73], &42i64.to_le_bytes());
        // cycles at offset 73 (u64 LE)
        assert_eq!(&bytes[73..81], &12345u64.to_le_bytes());
    }
}
