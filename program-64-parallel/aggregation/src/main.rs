//! CKB-zkVM Aggregation Guest Program
//!
//! This SP1 program verifies N per-script proofs (from guest-single) and
//! commits a single aggregated PublicValues output — identical in format
//! to the monolithic batched proof.
//!
//! ## How It Works
//!
//! 1. Reads verification keys and public values for each per-script proof
//! 2. For each proof: computes SHA256(public_values), calls verify_sp1_proof
//! 3. Decodes SingleScriptPublicValues from each proven public values
//! 4. Validates all proofs are for the same transaction
//! 5. Commits aggregated PublicValues (same format as monolithic proof)
//!
//! ## SP1 Recursive Verification
//!
//! Uses SP1's deferred proof verification: verify_sp1_proof accumulates a
//! Poseidon2 digest of all verified proofs. The prover checks this digest
//! against the actual proofs supplied via stdin.write_proof on the host side.

#![no_main]
sp1_zkvm::entrypoint!(main);

use sha2::{Digest, Sha256};

use ckb_zkvm_lib::{PublicValues, ScriptExecutionResult};
use ckb_zkvm_lib::SingleScriptPublicValues;

/// Main entry point for the aggregation guest program.
pub fn main() {
    // Step 1: Read verification keys (one per script proof).
    // Each vkey is the hash of the SP1 verifying key as 8 u32 words.
    let vkeys: Vec<[u32; 8]> = sp1_zkvm::io::read();

    // Step 2: Read raw public values bytes (one per script proof).
    let all_public_values: Vec<Vec<u8>> = sp1_zkvm::io::read();

    assert_eq!(
        vkeys.len(),
        all_public_values.len(),
        "vkeys count ({}) != public values count ({})",
        vkeys.len(),
        all_public_values.len()
    );
    assert!(!vkeys.is_empty(), "no proofs to aggregate");

    // Step 3: Verify each per-script proof and collect results.
    let mut tx_hash: Option<[u8; 32]> = None;
    let mut script_results = Vec::with_capacity(vkeys.len());

    for i in 0..vkeys.len() {
        // Compute SHA256 digest of the public values (required by verify_sp1_proof)
        let pv_digest = Sha256::digest(&all_public_values[i]);

        // Verify the per-script proof via SP1 deferred verification syscall.
        // This doesn't actually verify inline — it accumulates a running digest
        // that the SP1 prover checks against the supplied proofs.
        sp1_zkvm::lib::verify::verify_sp1_proof(&vkeys[i], &pv_digest.into());

        // Decode the proven public values
        let spv = SingleScriptPublicValues::from_bytes(&all_public_values[i])
            .expect("invalid per-script public values");

        // All proofs must be for the same transaction
        match &tx_hash {
            None => tx_hash = Some(spv.tx_hash),
            Some(expected) => {
                assert_eq!(
                    &spv.tx_hash, expected,
                    "tx_hash mismatch: proof {} has different tx_hash",
                    i
                );
            }
        }

        script_results.push(ScriptExecutionResult::new(
            spv.script_hash,
            spv.script_group_type,
            spv.exit_code,
            spv.cycles,
        ));
    }

    // Step 4: Commit aggregated public values.
    // Uses the SAME format as the monolithic batched proof — existing
    // verification tooling works unchanged.
    let pv = PublicValues::new_batched(
        tx_hash.expect("no proofs to aggregate"),
        script_results,
    );
    sp1_zkvm::io::commit_slice(&pv.to_bytes());
}
