//! CKB-zkVM Per-Script Guest Program (SP1 Hypercube v6, 64-bit)
//!
//! Proves a SINGLE CKB script group execution. Used in parallel proving:
//! each script group gets its own proof, then an aggregation guest verifies
//! all per-script proofs.

#![no_main]
sp1_zkvm::entrypoint!(main);

use ckb_zkvm_guest_common::replay_trace;
use ckb_zkvm_lib::ExecutionTrace;
use ckb_zkvm_lib::SingleScriptPublicValues;

/// Main entry point for the per-script SP1 guest program.
pub fn main() {
    // Read and deserialize a single execution trace.
    // Block scope ensures raw bytes are dropped before replay begins.
    let trace = {
        let trace_bytes: Vec<u8> = sp1_zkvm::io::read();
        ExecutionTrace::decode_from_slice(&trace_bytes)
            .expect("failed to deserialize execution trace")
    };

    // Extract metadata before replay consumes the trace.
    let tx_hash = trace
        .tx_hash_h256()
        .expect("failed to extract tx_hash from trace");
    let script_hash = trace
        .script_hash_h256()
        .expect("failed to extract script_hash from trace");
    assert!(
        trace.script_group_type <= 1,
        "invalid script_group_type: {} (expected 0=Lock or 1=Type)",
        trace.script_group_type
    );
    let script_group_type = trace.script_group_type as u8;

    // Replay through CKB-VM.
    replay_trace(trace);

    // Commit per-script public values.
    let pv = SingleScriptPublicValues::new(
        tx_hash,
        script_hash,
        script_group_type,
        0, // exit_code: 0 because replay_trace panics on non-zero
        0, // cycles: 0 because cycle counting is disabled in guest
    );
    sp1_zkvm::io::commit_slice(&pv.to_bytes());
}
