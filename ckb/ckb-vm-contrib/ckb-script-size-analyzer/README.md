# CKB Script Size Analyzer

A command-line tool for analyzing the size of symbols in CKB smart contract binaries. This tool helps developers understand the size distribution of functions and data structures in their compiled RISC-V binaries, enabling optimization of smart contracts for deployment on the CKB blockchain.

## Overview

CKB Script Size Analyzer parses ELF binaries and extracts symbol information including function names, variable names, and their respective sizes. It's particularly useful for:

- Identifying large functions that could be optimized
- Understanding memory usage patterns in smart contracts
- Debugging code bloat issues
- Optimizing contracts for CKB's resource constraints

## Usage

### Basic Usage

A riscv binary with debug information is required. For example, you can use the debug build of a CKB-VM test suite program:

```bash
$ ckb-script-size-analyzer --input ../ckb-vm-test-suite/programs/build/release/secp256k1_ecdsa_ckbvm.debug

Symbol: _Exit, size: 50
Symbol: ___errno_location, size: 8
Symbol: __addtf3, size: 1050
Symbol: __aio_close, size: 2
Symbol: __ckb_hijack_brk, size: 56
... ...
Symbol: wcrtomb, size: 260
Symbol: wctomb, size: 20
Symbol: xdigits, size: 16
```

All symbols sorted by name. If you want to sort by size, use the `--sort size` option:

```bash
$ ckb-script-size-analyzer --input ../ckb-vm-test-suite/programs/build/release/secp256k1_ecdsa_ckbvm.debug --sort size
```

### Command Line Options

```bash
Usage: ckb-script-size-analyzer [OPTIONS] --input <INPUT>

Options:
      --input <INPUT>  Input file
      --sort <SORT>    Symbol sort mode [default: name] [possible values: name, size]
      --function-only  Function only mode
  -h, --help           Print help (see more with '--help')
  -V, --version        Print version
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
