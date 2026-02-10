//! Build script for compiling the 64-bit guest program to RISC-V ELF.
//!
//! This uses sp1-build v6 to compile the guest program in program-64/ to a
//! 64-bit RISC-V ELF binary that can be executed inside SP1 Hypercube.

fn main() {
    // Rerun if guest program or shared guest code changes
    println!("cargo:rerun-if-changed=../program-64/src/main.rs");
    println!("cargo:rerun-if-changed=../program-64/Cargo.toml");
    println!("cargo:rerun-if-changed=../guest-common/src/lib.rs");
    println!("cargo:rerun-if-changed=../guest-common/Cargo.toml");

    // Build the SP1 guest program (64-bit RISC-V target)
    sp1_build::build_program("../program-64");
}
