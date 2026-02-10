# ckb-zkvm

Zero-knowledge proof generation for CKB transactions using SP1 zkVM.

## How It Works

The host fetches a confirmed CKB transaction via RPC, re-executes it locally to collect syscall traces, then passes those traces to an SP1 guest program. The guest replays the trace through CKB-VM using recorded syscall responses (not live execution), and commits the transaction hash and script results as public values. SP1 then generates a succinct proof that the replay was correct.

```
Host                              Guest (SP1 zkVM)
─────                             ────────────────
Fetch TX via RPC                  Read serialized traces
Execute all script groups         Deserialize protobuf
Record syscall traces             For each script group:
Serialize to protobuf               Load ELF into CKB-VM
Pass to SP1                         Replay with recorded syscalls
                                    Assert exit_code == 0
                                  Commit tx_hash + results
```

**Trace replay, not direct execution.** Running CKB-VM directly inside SP1 with live syscalls causes OOM. The replay approach returns pre-recorded responses, making it efficient and deterministic.

## Project Structure

```
ckb-zkvm/
├── lib/                          # Shared types (ExecutionTrace, PublicValues, protobuf)
├── guest-common/                 # Shared guest replay code (replay_trace)
├── host-common/                  # Shared executor + RPC client
├── program/                      # SP1 guest — 32-bit RISC-V (SP1 v4)
├── program-64/                   # SP1 guest — 64-bit RISC-V (SP1 v6 Hypercube)
├── program-64-parallel/
│   ├── script-prover/            # SP1 v6 per-script guest
│   └── aggregation/              # SP1 v6 aggregation guest
├── host/                         # Host binaries (SP1 v4): prove, verify, vkey, scan
├── host-64/                      # Host binaries (SP1 v6): prove-v6
├── host-64-parallel/             # Host binaries (SP1 v6 parallel): prove-parallel, verify-parallel
└── ckb/                          # Local forks: ckb-vm, ckb-vm-contrib, ckb-standalone-debugger
```

Three provers are available:

| Prover | Directory | SP1 Version | Architecture | Approach |
|--------|-----------|-------------|--------------|----------|
| **v4** | `host/` | SP1 v4 | 32-bit RISC-V | Monolithic (all scripts in one proof) |
| **v6** | `host-64/` | SP1 v6 Hypercube | 64-bit RISC-V | Monolithic (all scripts in one proof) |
| **v6 Parallel** | `host-64-parallel/` | SP1 v6 Hypercube | 64-bit RISC-V | Per-script proofs + aggregation |

`host/` uses the root workspace. `host-64/` and `host-64-parallel/` are separate workspaces because SP1 v4 and v6 have incompatible dependency trees.

## Prerequisites

- Rust nightly
- [SP1 toolchain](https://docs.succinct.xyz/getting-started/install.html) (`sp1up`)
- CKB RPC access (testnet: `https://testnet.ckb.dev/rpc`, mainnet: `https://mainnet.ckb.dev/rpc`)

## Build

```bash
# Build v4 (32-bit) — compiles guest ELF + host binaries
cargo build --release

# Build v6 (64-bit) — separate workspace
cd host-64 && cargo build --release

# Build v6 parallel — separate workspace (also compiles both guest programs)
cd host-64-parallel && cargo build --release
```

The build process compiles the guest program to a RISC-V ELF binary via `build.rs`, then builds the host binaries that embed/reference it.

## Usage

### SP1 v4 (32-bit)

```bash
# Execute (no proof) — trace collection + SP1 guest execution
SP1_PROVER=mock cargo run --release --bin prove -- <TX_HASH> --no-prove --rpc <RPC_URL>

# Generate a proof
cargo run --release --bin prove -- <TX_HASH> --rpc <RPC_URL> -o ./output

# Verify the proof
cargo run --release --bin verify -- ./output/proof.bin
cargo run --release --bin verify -- ./output/proof.bin --expected-hash <TX_HASH>
```

### SP1 v6 (64-bit)

```bash
cd host-64

# Execute (no proof)
SP1_PROVER=mock cargo run --release --bin prove-v6 -- <TX_HASH> --no-prove --rpc <RPC_URL>

# Generate a proof
cargo run --release --bin prove-v6 -- <TX_HASH> --rpc <RPC_URL> -o ./output
```

### SP1 v6 Parallel

The parallel prover generates per-script compressed proofs concurrently, then aggregates them into a single proof. This is faster for transactions with many script groups.

```bash
cd host-64-parallel

# Execute (no proof) — runs each script group through SP1 in parallel
SP1_PROVER=mock cargo run --release --bin prove-parallel -- <TX_HASH> --no-prove --rpc <RPC_URL>

# Generate a proof (prove + aggregate)
cargo run --release --bin prove-parallel -- <TX_HASH> --rpc <RPC_URL> -o ./output

# Verify the aggregated proof
cargo run --release --bin verify-parallel -- ./output/proof.bin
cargo run --release --bin verify-parallel -- ./output/proof.bin --expected-hash <TX_HASH>
```

**Prove + verify in sequence:**

```bash
cd host-64-parallel

# Step 1: Generate proof (outputs proof.bin, vkey.bin, public_values.json)
cargo run --release --bin prove-parallel -- <TX_HASH> --rpc <RPC_URL> -o ./output

# Step 2: Verify the proof (auto-discovers vkey.bin next to proof.bin)
cargo run --release --bin verify-parallel -- ./output/proof.bin
```

Options for `prove-parallel`:

| Flag | Default | Description |
|------|---------|-------------|
| `--rpc <URL>` | `https://testnet.ckb.dev/rpc` | CKB RPC endpoint |
| `-o, --output <PATH>` | `./output` | Output directory for artifacts |
| `--no-prove` | — | Run SP1 guest execution without proof generation |
| `--execute-only` | — | Skip SP1 entirely (host-side trace collection only) |
| `--max-parallel <N>` | `4` | Max concurrent SP1 proving tasks |

### Other Tools

```bash
# Extract verification key (v4)
cargo run --release --bin vkey -- --format json --output ./output/vkey.json
cargo run --release --bin vkey -- --print-hash

# Scan blocks — replay all transactions through Host → SP1 execution
cargo run --release --bin scan -- 5000 --count 100 --rpc https://mainnet.ckb.dev/rpc
```

`--no-prove` runs trace collection + SP1 guest execution without generating a proof. Use `--execute-only` to skip SP1 entirely (host-side trace collection only).

## Benchmark Results

All transactions tested across SP1 v4 (32-bit), SP1 v6 (64-bit), and SP1 v6 Parallel:

| # | Type | Transaction | Network | SP1 v4 (32-bit) | SP1 v6 (64-bit) | v6 vs v4 |
|---|------|-------------|---------|----------------:|----------------:|---------:|
| 1 | Exec (Taproot) | `0x6e1bc6dd..` | Testnet | 290,786,895 | 243,716,184 | -16.2% |
| 2 | Spawn (Fiber) | `0x562a1e35..` | Testnet | 348,487,939 | 297,474,428 | -14.6% |
| 3 | Single script | `0x3089e8de..` | Testnet | 328,080,154 | 279,637,209 | -14.8% |
| 4 | Omnilock (4 groups) | `0x69800933..` | Testnet | 2,036,374,393 | 1,740,634,569 | -14.5% |
| 5 | 91 inputs | `0xacb5dd54..` | Mainnet | 329,179,781 | 280,982,394 | -14.6% |
| 6 | DAO | `0xc73693bd..` | Mainnet | 315,348,698 | 268,934,689 | -14.7% |
| 7 | JoyId spawn (8 groups) | `0x42713ecf..` | Mainnet | 1,881,277,130 | 1,599,616,216 | -15.0% |
| 8 | Standard (7 groups) | `0x07cd86df..` | Mainnet | 427,686,773 | 363,382,926 | -15.0% |
| 9 | Large tx (59 groups) | `0xe6a9ace8..` | Mainnet | 8,897,988,973 | 7,556,736,845 | -15.1% |

The 64-bit SP1 v6 Hypercube target provides a consistent ~14.5-16% cycle reduction across all transaction types. The parallel prover produces identical total cycles to the monolithic v6 prover but executes script groups concurrently, reducing wall-clock time for multi-group transactions.

## Exec and Spawn Handling

CKB scripts can use two syscalls that create additional VM instances:

### Exec (syscall 2043)

`exec` replaces the current running program with a new binary (similar to Unix exec). The collector records syscall traces for each "generation" (gen 0 = original program, gen 1+ = exec'd programs). Only gen 0's trace is passed to the guest; gen 1+ entries are discarded.

**In the guest**, the `ExecHaltingSyscalls` wrapper intercepts the exec syscall and halts the VM with exit code 0 instead of loading a new binary. Since the VM halts immediately on exec, only gen 0's syscalls (up to the exec call) are ever consumed. This is safe because:
- The on-chain CKB consensus already verified the full exec chain
- Gen 0's trace is self-contained — it covers all syscalls before the exec call

### Spawn (syscall 2101)

`spawn` creates a child VM that runs concurrently with the parent. The collector records traces keyed by `(vm_id, generation_id)`.

**In the guest**, only the parent VM trace (vm_id=0) is replayed. Child VM traces are not re-executed because:
- The parent's spawn syscall response already contains the child's exit code
- The parent trace is self-contained — replaying it with recorded responses is sufficient
- Re-executing child VMs would multiply the proof cost with no additional security

### Summary

Both exec and spawn rely on the same principle: **the on-chain CKB node already verified all VM executions**. The guest only needs to prove that the recorded trace replays correctly through CKB-VM, not that the syscall responses themselves are correct. Trace correctness is a trust assumption on the host-side collector.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `SP1_PROVER` | `mock` | `mock` for testing, `cpu`/`cuda` for real proofs, `network` for SP1 network |
| `CKB_RPC_URL` | `https://testnet.ckb.dev/rpc` | CKB node RPC endpoint |
| `OUTPUT_DIR` | `./output` | Directory for proof artifacts |

## License

MIT
