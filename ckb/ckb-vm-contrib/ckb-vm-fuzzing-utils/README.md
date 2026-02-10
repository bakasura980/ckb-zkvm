# CKB VM Fuzzing Utils

A comprehensive fuzzing utilities library for testing and validating CKB-VM (RISC-V virtual machine) implementations. This library provides essential tools and abstractions for building robust fuzz testing frameworks for CKB smart contracts and the VM itself.

## Core Components

### `SynchronousSyscalls<S, M>`

The main syscall handler that implements all CKB system calls in a synchronous manner. It bridges the gap between the `SyscallImpls` trait (designed for script usage) and VM-specific requirements.

### `CkbvmRunnerImpls<Mac: SupportMachine>`

An extension trait that augments `SyscallImpls` with additional VM-specific functionality, particularly for operations like `load_cell_code` that require direct access to VM internals.

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
