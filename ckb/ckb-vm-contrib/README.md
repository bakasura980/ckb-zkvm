# CKB-VM-Contrib

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.85.0-blue.svg)](https://www.rust-lang.org/)

Community-contributed tools, extensions, testing and experimental features for the [CKB-VM](https://github.com/nervosnetwork/ckb-vm) (RISC-V virtual machine).

## Overview

This repository contains a collection of utilities, testing frameworks, and development tools for the CKB-VM ecosystem. These tools are designed to help developers build, test, debug, and analyze smart contracts running on the CKB blockchain.

## Components

- **[ckb-mock-tx-types](./ckb-mock-tx-types)** - Data types for mocking CKB transaction environments
- **[ckb-script-size-analyzer](./ckb-script-size-analyzer)** - Analyze and optimize smart contract binary sizes
- **[ckb-vm-fuzzing-utils](./ckb-vm-fuzzing-utils)** - Fuzzing utilities for VM testing
- **[ckb-vm-syscall-tracer](./ckb-vm-syscall-tracer)** - Trace and analyze syscall usage in CKB scripts
- **[ckb-vm-test-suite](./ckb-vm-test-suite)** - Comprehensive test suite for CKB-VM
- **[ckb-x64-simulator](./ckb-x64-simulator)** - x64 simulator for CKB smart contracts with native tooling support
- **[protobuf-ckb-syscalls](./protobuf-ckb-syscalls)** - Protocol buffer definitions for CKB syscalls
- **[spike-sys](./spike-sys)** - Rust bindings for the RISC-V ISA simulator

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
