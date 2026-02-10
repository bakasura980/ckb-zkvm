//! Build script for compiling both guest programs to RISC-V ELF.
//!
//! Builds:
//! 1. program-64-parallel: per-script replay guest (no verify feature)
//! 2. program-64-aggregation: proof aggregation guest (with verify feature)

fn main() {
    // Rerun if guest programs or shared guest code changes
    println!("cargo:rerun-if-changed=../program-64-parallel/script-prover/src/main.rs");
    println!("cargo:rerun-if-changed=../program-64-parallel/script-prover/Cargo.toml");
    println!("cargo:rerun-if-changed=../program-64-parallel/aggregation/src/main.rs");
    println!("cargo:rerun-if-changed=../program-64-parallel/aggregation/Cargo.toml");
    println!("cargo:rerun-if-changed=../guest-common/src/lib.rs");
    println!("cargo:rerun-if-changed=../guest-common/Cargo.toml");

    // Build per-script guest
    sp1_build::build_program("../program-64-parallel/script-prover");

    // Build aggregation guest
    sp1_build::build_program("../program-64-parallel/aggregation");
}
