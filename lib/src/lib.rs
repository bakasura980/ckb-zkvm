//! # ckb-zkvm-lib
//!
//! Shared library for ckb-zkvm containing:
//! - Execution trace types (protobuf-generated)
//! - Public values for proof commitment
//! - Serialization utilities
//! - Error types

pub mod error;
pub mod public_values;
pub mod single_script;
pub mod trace;

// Re-export main types at crate root
pub use error::Error;
pub use public_values::{PublicValues, ScriptExecutionResult};
pub use single_script::{SingleScriptPublicValues, SINGLE_SCRIPT_PV_SIZE};
pub use trace::{
    binary_locator, syscall, BinaryLocator, CellDataLocator, DepGroupLocator, ExecutionTrace,
    Fds, IoData, ScriptResult, Syscall, Syscalls, Terminated, TransactionTraces, VmCreation,
};

/// Result type alias using our Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// 32-byte hash type (CKB transaction hash).
pub type H256 = [u8; 32];

/// ELF magic bytes (first 4 bytes of any valid ELF binary).
pub const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

/// Parse a hex string into H256.
pub fn parse_h256(hex_str: &str) -> Result<H256> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    if hex_str.len() != 64 {
        return Err(Error::InvalidHash(format!(
            "expected 64 hex chars, got {}",
            hex_str.len()
        )));
    }

    let bytes = hex::decode(hex_str).map_err(|e| Error::InvalidHash(e.to_string()))?;

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

/// Format H256 as hex string with 0x prefix.
pub fn format_h256(hash: &H256) -> String {
    format!("0x{}", hex::encode(hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_h256_with_prefix() {
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let hash = parse_h256(hex_str).unwrap();
        assert_eq!(hash[31], 1);
    }

    #[test]
    fn test_parse_h256_without_prefix() {
        let hex_str = "0000000000000000000000000000000000000000000000000000000000000001";
        let hash = parse_h256(hex_str).unwrap();
        assert_eq!(hash[31], 1);
    }

    #[test]
    fn test_format_h256() {
        let mut hash = [0u8; 32];
        hash[31] = 1;
        let formatted = format_h256(&hash);
        assert_eq!(
            formatted,
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );
    }

    #[test]
    fn test_parse_h256_invalid_length() {
        let result = parse_h256("0x1234");
        assert!(result.is_err());
    }
}
