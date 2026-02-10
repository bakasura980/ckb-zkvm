//! Shared host-side library for CKB transaction execution and trace generation.
//!
//! This crate provides the SP1-agnostic infrastructure used by all CKB-zkVM
//! host crates (`script`, `script-v6`, `parallel-zkVM/host`):
//! - CKB RPC client for fetching transaction data
//! - Transaction executor for generating execution traces

pub mod executor;
pub mod rpc;

pub use executor::TransactionExecutor;
pub use rpc::{CkbRpcClient, RpcConfig};
