# AGENTS.md - AI Agent Instructions for ckb-vm-contrib

This file provides guidance for AI coding agents working in this repository.

## Project Overview

CKB-VM-Contrib is a Rust workspace containing community-contributed tools, extensions, 
and testing frameworks for [CKB-VM](https://github.com/nervosnetwork/ckb-vm), a RISC-V 
virtual machine used by the Nervos CKB blockchain.

### Workspace Members
- `ckb-mock-tx-types` - Mock CKB transaction types for testing
- `ckb-script-size-analyzer` - Smart contract binary size analyzer
- `ckb-vm-fuzzing-utils` - Fuzzing utilities for VM testing
- `ckb-vm-syscall-tracer` - Syscall tracing and analysis
- `ckb-vm-test-suite` - Comprehensive VM test suite and benchmarks
- `ckb-x64-simulator` - x64 simulator for CKB smart contracts
- `protobuf-ckb-syscalls` - Protocol buffer definitions for syscalls

---

## Build Commands

### Prerequisites
- Rust 1.85.0 (pinned in `rust-toolchain.toml`)
- RISC-V target: `rustup target add riscv64imac-unknown-none-elf`
- Clang (for C programs)
- RISC-V GCC toolchain (for test programs)

### Build Workspace
```bash
cargo build                          # Build all workspace members
cargo build -p ckb-mock-tx-types     # Build specific package
cargo build --release                # Release build
```

### Build Test Programs (required before running tests)
```bash
cd ckb-vm-test-suite/programs && make build
```

---

## Testing Commands

### Run All Tests
```bash
cargo test                           # Run all workspace tests
cargo test -p ckb-vm-test-suite      # Test specific package
```
### Run Single Test
```bash
cargo test test_name                 # Run test by name
cargo test test_ed25519              # Example: run ed25519 test
cargo test -p ckb-vm-test-suite test_ed25519  # Single test in package
```
### Full Test Suite (includes RISC-V compliance tests)
```bash
cd ckb-vm-test-suite && bash test.sh
```
### Benchmarks
```bash
cd ckb-vm-test-suite && cargo bench  # Run all benchmarks
cargo bench --bench algorithm        # Run specific benchmark
```

---

## Linting and Formatting

```bash
cargo fmt                            # Format all code
cargo fmt -- --check                 # Check formatting without changes
cargo clippy                         # Run clippy lints
cargo clippy -p ckb-x64-simulator    # Lint specific package
cargo clippy -- -D warnings          # Treat warnings as errors
```
### Formatting Configuration (`rustfmt.toml`)
- `max_width = 120` - Maximum line width
- `use_small_heuristics = "Max"` - Use maximum width for all constructs

---

## Code Style Guidelines

### Imports
Order imports in groups: 1) std, 2) external crates, 3) crate-local (`crate::`, `super::`)

### Naming Conventions
- Types/Traits: `PascalCase` (e.g., `MockTransaction`, `ResourceLoader`)
- Functions/Methods: `snake_case` (e.g., `get_live_cell`, `build_verifier`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `CKB_SUCCESS`, `SOURCE_INPUT`)
- Modules: `snake_case` (e.g., `readonly_machines`, `simulator_context`)
- Type parameters: Single uppercase letters (e.g., `M`, `DL`, `F`)

### Error Handling
- Use `Result<T, Error>` for fallible operations
- Map errors with descriptive context using `map_err`
- Use `?` operator for error propagation
- Use `ok_or` / `ok_or_else` to convert Options to Results

```rust
let resource = Resource::from_mock_tx(mock_tx).map_err(Error::External)?;
let resolved = resolve_transaction(tx, &mut set, &resource, &resource)
    .map_err(|e| Error::External(format!("resolving error: {}", e)))?;
```

### Pattern Matching
Use exhaustive match statements; avoid catch-all `_` when possible:
```rust
match source {
    SOURCE_INPUT => { /* ... */ }
    SOURCE_OUTPUT => { /* ... */ }
    SOURCE_CELL_DEP => { /* ... */ }
    _ => panic!("Invalid source: {}", source),
}
```

### Documentation
- Add doc comments (`///`) for public items
- Use module-level docs (`//!`) for module overview

---

## FFI and Unsafe Code

This codebase interfaces with C code and uses FFI:
```rust
#[unsafe(no_mangle)]
pub extern "C" fn ckb_exit(code: i8) -> i32 {
    std::process::exit(code.into());
}
```
- Minimize unsafe code scope
- Document safety invariants
- Use helper functions to encapsulate unsafe operations

---

## Project-Specific Notes

### CKB-VM Modes
- `interpreter32` / `interpreter64` - Interpreted execution
- `asm64` - JIT-compiled execution (faster)
- `mop` - Macro-op fusion optimization

### RISC-V ISA Extensions
- `ISA_IMC` - Base integer + multiply + compressed
- `ISA_B` - Bit manipulation extension
- `ISA_MOP` - Macro-op fusion

### Git Submodules
Dependencies in `deps/` are git submodules. Initialize with:
```bash
git submodule update --init --recursive
```
