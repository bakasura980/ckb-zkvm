//! # ckb-zkvm-host
//!
//! Host-side SP1 v4 proving binaries for CKB transactions.
//!
//! Executor and RPC functionality is provided by the shared
//! `ckb-zkvm-host-common` crate.

pub use ckb_zkvm_host_common::executor;
pub use ckb_zkvm_host_common::rpc;
pub use ckb_zkvm_host_common::TransactionExecutor;
pub use ckb_zkvm_host_common::{CkbRpcClient, RpcConfig};
