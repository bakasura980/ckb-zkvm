# Optimized Library for CKB-VM

This project focuses on optimizing various algorithms specifically for [CKB-VM](https://github.com/nervosnetwork/ckb-vm).

## Optimization Philosophy

This project leverages specific RISC-V extensions supported by CKB-VM (version 2) to achieve better performance, including the **B extension** (Zba, Zbb, Zbc, Zbs).

While CKB-VM targets the RISC-V instruction set, general-purpose optimizations used on other platforms may not always be effective here. For example:

- **Memory Hierarchy**: CKB-VM does not have L1/L2 caches, so localized memory access patterns do not provide the same performance benefits as they do on traditional CPUs.
- **Cycles**: Performance is measured based on [CKB-VM cycles](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0014-vm-cycle-limits/0014-vm-cycle-limits.md).
- **MOP Fusion**: CKB-VM implements [Micro-operation (MOP) fusion](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0033-ckb-vm-version-1/0033-ckb-vm-version-1.md#42-mop) for certain instruction sequences. These patterns differ from those in traditional CPUs and directly impact cycle consumption.

## Implementations

The following algorithms are currently implemented:

- [sha256](./sha256)
