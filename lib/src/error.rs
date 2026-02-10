//! Error types for ckb-zkvm.

use thiserror::Error;

/// Unified error type for ckb-zkvm operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid transaction hash format.
    #[error("invalid hash: {0}")]
    InvalidHash(String),

    /// Invalid execution trace.
    #[error("invalid trace: {0}")]
    InvalidTrace(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// CKB RPC error.
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Transaction not found.
    #[error("transaction not found: {0}")]
    TxNotFound(String),

    /// Cell not found.
    #[error("cell not found: {0}")]
    CellNotFound(String),

    /// Transaction execution error.
    #[error("execution error: {0}")]
    Execution(String),

    /// Proof generation error.
    #[error("proving error: {0}")]
    Proving(String),

    /// Proof verification error.
    #[error("verification error: {0}")]
    Verification(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create an RPC error with context.
    pub fn rpc(msg: impl Into<String>) -> Self {
        Self::Rpc(msg.into())
    }

    /// Create an execution error with context.
    pub fn execution(msg: impl Into<String>) -> Self {
        Self::Execution(msg.into())
    }

    /// Create a proving error with context.
    pub fn proving(msg: impl Into<String>) -> Self {
        Self::Proving(msg.into())
    }

    /// Create a verification error with context.
    pub fn verification(msg: impl Into<String>) -> Self {
        Self::Verification(msg.into())
    }
}
