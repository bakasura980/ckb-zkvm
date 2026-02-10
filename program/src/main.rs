//! CKB-zkVM Guest Program (SP1 v4, 32-bit)
//!
//! Proves ALL scripts in a single proof by iterating through the
//! TransactionTraces and replaying each script via CKB-VM.

#![no_main]
sp1_zkvm::entrypoint!(main);

use ckb_zkvm_guest_common::replay_trace;
use ckb_zkvm_lib::{PublicValues, ScriptExecutionResult, TransactionTraces, H256};

/// Main entry point for the SP1 guest program.
pub fn main() {
    // Read and deserialize traces, then free the raw bytes.
    // The block ensures trace_bytes is dropped before replay begins,
    // avoiding 2x peak memory (raw bytes + deserialized structs).
    let tx_traces = {
        let trace_bytes: Vec<u8> = sp1_zkvm::io::read();
        TransactionTraces::decode_from_slice(&trace_bytes)
            .expect("failed to deserialize transaction traces")
    };

    let (tx_hash, script_results) = process_traces(tx_traces);

    let public_values = PublicValues::new_batched(tx_hash, script_results);
    sp1_zkvm::io::commit_slice(&public_values.to_bytes());
}

/// Process transaction traces — replay each script and collect results.
/// Takes ownership of the traces to avoid cloning ELF binaries during replay.
fn process_traces(tx_traces: TransactionTraces) -> (H256, Vec<ScriptExecutionResult>) {
    let tx_hash: H256 = tx_traces
        .tx_hash_h256()
        .expect("failed to extract tx_hash from traces");

    let mut script_results = Vec::with_capacity(tx_traces.script_count());

    for trace in tx_traces.script_traces {
        let script_hash = trace.script_hash_h256()
            .expect("failed to extract script_hash from trace");
        assert!(
            trace.script_group_type <= 1,
            "invalid script_group_type: {} (expected 0=Lock or 1=Type)",
            trace.script_group_type
        );
        let script_group_type = trace.script_group_type as u8;

        replay_trace(trace);

        script_results.push(ScriptExecutionResult::new(
            script_hash,
            script_group_type,
            0,
            0,
        ));
    }

    (tx_hash, script_results)
}
