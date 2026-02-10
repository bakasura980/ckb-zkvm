//! # ckb-zkvm-host-64
//!
//! Host-side SP1 Hypercube v6 (64-bit) proving binaries for CKB transactions.
//!
//! Executor and RPC functionality is provided by the shared
//! `ckb-zkvm-host-common` crate.

pub use ckb_zkvm_host_common::executor;
pub use ckb_zkvm_host_common::rpc;
pub use ckb_zkvm_host_common::TransactionExecutor;
pub use ckb_zkvm_host_common::{CkbRpcClient, RpcConfig};
