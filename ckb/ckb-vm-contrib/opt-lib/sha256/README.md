# Optimized SHA-256 for CKB-VM

This folder contains an optimized implementation of SHA-256 for CKB-VM.

## Optimizations

- **Loop Unrolling**: Uses `#pragma GCC unroll` to improve throughput.
- **RISC-V B Extension**: Uses `rev8` (via `__builtin_bswap64`) for efficient byte swapping.
- **64-bit State Loading**: Loads the internal state using 64-bit operations to reduce instructions.
- **LTO**: Makefile enables Link Time Optimization (`-flto`).

## Build & Run

### Setup Dependencies

```sh
$ cd $ROOT/deps/musl
$ CLANG=clang-19 bash ckb/build.sh

$ cd $ROOT/deps/compiler-rt-builtins-riscv
$ make CC=clang-19 AR=llvm-ar-19
```

### Build SHA-256

```sh
make
```

### Run Tests

```sh
make run
```
