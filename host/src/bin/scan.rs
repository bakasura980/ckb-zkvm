// Lyubo: Move to a Readme file.
//! CKB-zkVM Block Scanner
//!
//! Scans a range of blocks and replays every transaction through
//! Host trace collection + SP1 guest execution (no proof).
//!
//! Usage:
//!   scan <START_BLOCK> [--count 100] [--rpc <URL>] [--elf <PATH>]

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use sp1_sdk::SP1Stdin;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use ckb_zkvm_lib::PublicValues;
use ckb_zkvm_host::executor::TransactionExecutor;
use ckb_zkvm_host::rpc::{CkbRpcClient, RpcConfig};

/// CLI arguments for the scan command.
#[derive(Parser, Debug)]
#[command(name = "scan")]
#[command(about = "Scan CKB blocks and replay transactions through SP1")]
struct Args {
    /// Starting block number
    #[arg(value_name = "START_BLOCK")]
    start_block: u64,

    /// Number of blocks to scan
    #[arg(long, default_value = "100")]
    count: u64,

    /// Path to the guest program ELF binary
    #[arg(
        long,
        env = "GUEST_ELF_PATH",
        default_value = "./target/elf-compilation/riscv32im-succinct-zkvm-elf/release/ckb-zkvm-program"
    )]
    elf: PathBuf,

    /// CKB RPC endpoint URL
    #[arg(long, env = "CKB_RPC_URL", default_value = "https://mainnet.ckb.dev/rpc")]
    rpc: String,
}

/// Result status for a single transaction.
enum TxStatus {
    Success {
        scripts: String,
        sp1_cycles: u64,
        sp1_time_ms: u64,
    },
    Failed {
        error: String,
    },
}

/// Result for a single transaction.
struct TxResult {
    block_number: u64,
    tx_hash: String,
    status: TxStatus,
}

/// Wait until a block is available on chain.
async fn wait_for_block(rpc: &CkbRpcClient, block_number: u64) -> Result<()> {
    loop {
        let tip = rpc
            .get_tip_block_number()
            .await
            .context("failed to get tip block number")?;

        if tip >= block_number {
            return Ok(());
        }

        info!(
            "block #{} not yet mined (tip={}), waiting 10s...",
            block_number, tip
        );
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

/// Collect traces for a transaction via the host executor.
/// Returns (scripts_summary, trace_bytes, script_count) on success.
async fn collect_traces(
    executor: &TransactionExecutor,
    tx_hash_hex: &str,
) -> Result<(String, Vec<u8>, usize), String> {
    let tx_hash = ckb_zkvm_lib::parse_h256(tx_hash_hex)
        .map_err(|e| format!("invalid tx hash: {}", e))?;

    let tx_traces = executor
        .execute_all(tx_hash)
        .await
        .map_err(|e| format!("trace collection failed: {}", e))?;

    let script_count = tx_traces.script_count();
    let mut lock_count = 0u32;
    let mut type_count = 0u32;
    for trace in &tx_traces.script_traces {
        if trace.script_group_type == 0 {
            lock_count += 1;
        } else {
            type_count += 1;
        }
    }
    let scripts = format!("Lock={}, Type={}", lock_count, type_count);
    let trace_bytes = tx_traces.encode_to_vec();

    Ok((scripts, trace_bytes, script_count))
}

fn format_cycles(cycles: u64) -> String {
    let s = cycles.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_time(ms: u64) -> String {
    if ms < 1_000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1_000.0)
    }
}

fn truncate_hash(hash: &str) -> String {
    if hash.len() > 14 {
        format!("{}...{}", &hash[..8], &hash[hash.len() - 4..])
    } else {
        hash.to_string()
    }
}

fn print_results(results: &[TxResult]) {
    // Table header
    println!();
    println!(
        "{:<8} {:<18} {:<22} {:>13} {:>10} {:>8}",
        "Block", "Transaction", "Scripts", "SP1 Cycles", "SP1 Time", "Status"
    );
    println!("{}", "-".repeat(83));

    for r in results {
        match &r.status {
            TxStatus::Success {
                scripts,
                sp1_cycles,
                sp1_time_ms,
            } => {
                println!(
                    "{:<8} {:<18} {:<22} {:>13} {:>10} {:>8}",
                    r.block_number,
                    truncate_hash(&r.tx_hash),
                    scripts,
                    format_cycles(*sp1_cycles),
                    format_time(*sp1_time_ms),
                    "OK"
                );
            }
            TxStatus::Failed { .. } => {
                println!(
                    "{:<8} {:<18} {:<22} {:>13} {:>10} {:>8}",
                    r.block_number,
                    truncate_hash(&r.tx_hash),
                    "",
                    "",
                    "",
                    "Failed"
                );
            }
        }
    }
    println!("{}", "-".repeat(83));

    // Summary
    let success_count = results
        .iter()
        .filter(|r| matches!(r.status, TxStatus::Success { .. }))
        .count();
    let fail_count = results.len() - success_count;

    println!();
    println!(
        "Summary: {}/{} transactions succeeded, {} failed",
        success_count,
        results.len(),
        fail_count
    );

    // Print failed transaction details
    let failures: Vec<&TxResult> = results
        .iter()
        .filter(|r| matches!(r.status, TxStatus::Failed { .. }))
        .collect();

    if !failures.is_empty() {
        println!();
        println!("Failed transactions:");
        for r in &failures {
            if let TxStatus::Failed { error } = &r.status {
                println!("  {} (block {}): {}", r.tx_hash, r.block_number, error);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let args = Args::parse();

    info!("CKB-zkVM Block Scanner");
    info!("========================");
    info!("Start block: {}", args.start_block);
    info!("Block count: {}", args.count);
    info!("RPC: {}", args.rpc);
    info!("ELF: {}", args.elf.display());

    // Load guest ELF
    let guest_elf = std::fs::read(&args.elf)
        .with_context(|| format!("failed to read guest ELF from {}", args.elf.display()))?;

    // Create RPC client and executor
    let rpc_config = RpcConfig {
        url: args.rpc.clone(),
        ..Default::default()
    };
    let rpc = CkbRpcClient::new(rpc_config).context("failed to create RPC client")?;
    let executor = TransactionExecutor::new(rpc);

    // Create a second RPC client for block/tip queries (executor owns the first)
    let rpc_config2 = RpcConfig {
        url: args.rpc.clone(),
        ..Default::default()
    };
    let block_rpc = CkbRpcClient::new(rpc_config2).context("failed to create block RPC client")?;

    // Initialize SP1 client (same pattern as prove.rs)
    let sp1_client = sp1_sdk::ProverClient::from_env();

    let mut results: Vec<TxResult> = Vec::new();
    let scan_start = Instant::now();

    let end_block = args.start_block + args.count;

    for block_number in args.start_block..end_block {
        // Wait for block to be mined
        wait_for_block(&block_rpc, block_number).await?;

        // Fetch block
        let block = block_rpc
            .get_block_by_number(block_number)
            .await
            .with_context(|| format!("failed to fetch block #{}", block_number))?;

        let tx_count = block.transactions.len();

        // Skip cellbase (tx[0]) — mining reward, no real inputs
        let user_txs = if tx_count > 1 {
            &block.transactions[1..]
        } else {
            &[]
        };

        info!(
            "Block #{}: {} transactions ({} user txs)",
            block_number,
            tx_count,
            user_txs.len()
        );

        for tx_view in user_txs {
            let tx_hash_hex = format!("0x{}", hex::encode(tx_view.hash.as_bytes()));

            info!("  Processing {}...", truncate_hash(&tx_hash_hex));

            // Step 1: Collect traces via host executor
            let status = match collect_traces(&executor, &tx_hash_hex).await {
                Err(error) => TxStatus::Failed { error },
                Ok((scripts, trace_bytes, script_count)) => {
                    // Step 2: Run SP1 guest execution
                    let mut stdin = SP1Stdin::new();
                    stdin.write(&trace_bytes);

                    let sp1_start = Instant::now();
                    match sp1_client.execute(&guest_elf, &stdin).run() {
                        Err(e) => TxStatus::Failed {
                            error: format!(
                                "SP1 execution failed ({} scripts): {}",
                                script_count, e
                            ),
                        },
                        Ok((public_values_raw, report)) => {
                            let sp1_time_ms = sp1_start.elapsed().as_millis() as u64;
                            let sp1_cycles = report.total_instruction_count();

                            // Step 3: Decode and verify public values
                            match PublicValues::from_bytes(public_values_raw.as_slice()) {
                                None => TxStatus::Failed {
                                    error: "failed to decode public values".to_string(),
                                },
                                Some(pv) => {
                                    if pv.exit_code != 0 {
                                        TxStatus::Failed {
                                            error: format!(
                                                "SP1 guest returned exit_code={}",
                                                pv.exit_code
                                            ),
                                        }
                                    } else {
                                        TxStatus::Success {
                                            scripts,
                                            sp1_cycles,
                                            sp1_time_ms,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            };

            match &status {
                TxStatus::Success {
                    scripts,
                    sp1_cycles,
                    sp1_time_ms,
                } => {
                    info!(
                        "    OK: {} | {} cycles | {}",
                        scripts,
                        format_cycles(*sp1_cycles),
                        format_time(*sp1_time_ms)
                    );
                }
                TxStatus::Failed { error } => {
                    warn!("    FAILED: {}", error);
                }
            }

            results.push(TxResult {
                block_number,
                tx_hash: tx_hash_hex,
                status,
            });
        }

        let block_ok = results.iter().rev().take(user_txs.len())
            .filter(|r| matches!(r.status, TxStatus::Success { .. }))
            .count();
        let block_fail = user_txs.len() - block_ok;
        info!(
            "Block #{} processed: {}/{} OK, {} failed",
            block_number, block_ok, user_txs.len(), block_fail
        );
    }

    let total_time = scan_start.elapsed();
    info!(
        "Scan complete: {} blocks, {} transactions in {:.1}s",
        args.count,
        results.len(),
        total_time.as_secs_f64()
    );

    print_results(&results);

    Ok(())
}
