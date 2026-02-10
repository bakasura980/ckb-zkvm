# Protobuf CKB Syscalls

A Rust library that provides Protocol Buffer format definitions and implementations for CKB syscalls, enabling serialization, replay, and analysis of CKB smart contract syscall interactions.

## Overview

The `protobuf-ckb-syscalls` library offers a standardized way to capture, serialize, and replay CKB VM syscall traces using Protocol Buffers.

The library defines several key message types in `traces.proto`.

## Usage

### Basic Syscall Implementation

```rust
use protobuf_ckb_syscalls::ProtobufImpls;
use ckb_std::syscalls::traits::SyscallImpls;

// Load syscalls from protobuf data
let impls = ProtobufImpls::new_with_bytes(&protobuf_data)
    .expect("Failed to parse protobuf data");

// Use with CKB syscalls
let result = impls.load_cell(&mut buffer, 0, 0, Source::Input);
```

### VM Runner Integration

```rust
use protobuf_ckb_syscalls::ProtobufVmRunnerImpls;
use ckb_vm::DefaultMachine;

// Create VM runner with protobuf syscalls.
let runner = ProtobufVmRunnerImpls::<DefaultMachine>::new_with_bytes(&data)
    .expect("Failed to create runner");

// Use with fuzzing or testing frameworks.
```

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
