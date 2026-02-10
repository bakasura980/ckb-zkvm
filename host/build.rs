//! Build script for compiling the guest program to RISC-V ELF.
//!
//! This uses sp1-build to compile the guest program in program/ to a
//! RISC-V ELF binary that can be executed inside SP1.

fn main() {
    // Rerun if guest program or shared guest code changes
    println!("cargo:rerun-if-changed=../program/src/main.rs");
    println!("cargo:rerun-if-changed=../program/Cargo.toml");
    println!("cargo:rerun-if-changed=../guest-common/src/lib.rs");
    println!("cargo:rerun-if-changed=../guest-common/Cargo.toml");

    // Build the SP1 guest program
    // Note: ckb-vm is patched to use rand 0.8 which supports RISC-V via getrandom 0.2
    sp1_build::build_program("../program");
}
