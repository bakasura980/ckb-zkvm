//! CKB-zkVM Parallel Proof Generation CLI (SP1 Hypercube v6 — 64-bit)
//!
//! Generates zero-knowledge proofs for CKB transactions using parallel
//! per-script-group proving with SP1 proof aggregation.
//!
//! Flow:
//! 1. Fetch transaction and generate traces (reuses script-v6 executor)
//! 2. For each script group in parallel: prove via guest-single → compressed proof
//! 3. Aggregate all compressed proofs via guest-aggregation → single proof
//!
//! Usage:
//!   prove-parallel <TX_HASH> [--rpc <URL>] [--output <PATH>]
//!                             [--no-prove] [--execute-only] [--max-parallel <N>]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use sp1_sdk::{
    Elf, HashableKey, ProveRequest, Prover, ProverClient, ProvingKey, SP1Proof,
    SP1ProofWithPublicValues, SP1Stdin,
};
use tokio::sync::Semaphore;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use ckb_zkvm_lib::{parse_h256, PublicValues};
use ckb_zkvm_lib::SingleScriptPublicValues;
use ckb_zkvm_host_common::executor::TransactionExecutor;
use ckb_zkvm_host_common::rpc::{CkbRpcClient, RpcConfig};
use ckb_zkvm_host_64_parallel::logging::{log_public_values, log_single_pv};

/// CLI arguments for the parallel prove command.
#[derive(Parser, Debug)]
#[command(name = "prove-parallel")]
#[command(about = "Generate a ZK proof for a CKB transaction using parallel per-script proving")]
struct Args {
    /// Transaction hash to prove (with or without 0x prefix)
    #[arg(value_name = "TX_HASH")]
    tx_hash: String,

    /// CKB RPC endpoint URL
    #[arg(long, env = "CKB_RPC_URL", default_value = "https://testnet.ckb.dev/rpc")]
    rpc: String,

    /// Output directory for proof artifacts
    #[arg(long, short, env = "OUTPUT_DIR", default_value = "./output")]
    output: PathBuf,

    /// Skip SP1 entirely (only run host-side trace collection)
    #[arg(long)]
    execute_only: bool,

    /// Run SP1 guest program without generating proofs
    #[arg(long)]
    no_prove: bool,

    /// Maximum number of parallel SP1 proving tasks
    #[arg(long, default_value = "4")]
    max_parallel: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let args = Args::parse();

    info!("CKB-zkVM Parallel Proof Generator (SP1 Hypercube v6, 64-bit)");
    info!("=============================================================");
    info!("Transaction: {}", args.tx_hash);
    info!("RPC: {}", args.rpc);
    info!("Max parallel: {}", args.max_parallel);

    // Load guest ELFs (embedded by build.rs)
    let single_elf: Arc<[u8]> = std::fs::read(env!("SP1_ELF_ckb-zkvm-program-64-parallel"))
        .context("failed to read guest-single ELF")?
        .into();
    let aggregation_elf: Arc<[u8]> =
        std::fs::read(env!("SP1_ELF_ckb-zkvm-program-64-aggregation"))
            .context("failed to read guest-aggregation ELF")?
            .into();

    info!(
        "Guest ELFs loaded: single={} bytes, aggregation={} bytes",
        single_elf.len(),
        aggregation_elf.len()
    );

    let tx_hash = parse_h256(&args.tx_hash).context("invalid transaction hash")?;
    std::fs::create_dir_all(&args.output).context("failed to create output directory")?;

    // Step 1: Fetch transaction and generate traces
    info!("Step 1: Fetching transaction and generating traces...");
    let start = Instant::now();

    let rpc_config = RpcConfig {
        url: args.rpc.clone(),
        ..Default::default()
    };
    let rpc = CkbRpcClient::new(rpc_config).context("failed to create RPC client")?;
    let executor = TransactionExecutor::new(rpc);

    let tx_traces = executor
        .execute_all(tx_hash)
        .await
        .context("failed to execute transaction")?;

    let script_count = tx_traces.script_count();
    info!(
        "Traces generated in {:?} ({} script groups, {} bytes total)",
        start.elapsed(),
        script_count,
        tx_traces.encode_to_vec().len()
    );

    if args.execute_only {
        info!("Execute-only mode: stopping after trace collection");
        return Ok(());
    }

    // Step 2: Split traces into individual ExecutionTraces
    info!("Step 2: Splitting traces into {} individual script traces...", script_count);
    let individual_traces: Vec<Vec<u8>> = tx_traces
        .script_traces
        .into_iter()
        .map(|trace| trace.encode_to_vec())
        .collect();

    // Step 3: Setup SP1 client
    info!("Step 3: Setting up SP1 client...");
    let client = ProverClient::from_env().await;

    if args.no_prove {
        // Execute-only mode: run each script through SP1 without generating proofs
        info!("Step 4: Executing {} script groups in parallel (no proof)...", script_count);
        let start = Instant::now();

        let semaphore = Arc::new(Semaphore::new(args.max_parallel));
        let client = Arc::new(client);
        let single_elf = single_elf.clone();

        let mut handles = Vec::with_capacity(individual_traces.len());

        for (i, trace_bytes) in individual_traces.into_iter().enumerate() {
            let sem = semaphore.clone();
            let client = client.clone();
            let elf = single_elf.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");

                let mut stdin = SP1Stdin::new();
                stdin.write(&trace_bytes);

                let elf = Elf::Dynamic(elf);
                let (public_values_raw, report) = client
                    .execute(elf, stdin)
                    .await
                    .with_context(|| format!("SP1 execution failed for script group {}", i))?;

                let spv = SingleScriptPublicValues::from_bytes(public_values_raw.as_slice())
                    .ok_or_else(|| anyhow!("failed to decode public values for script group {}", i))?;

                info!(
                    "  Script group {} executed: {} cycles (SP1)",
                    i,
                    report.total_instruction_count()
                );

                Ok::<(usize, SingleScriptPublicValues), anyhow::Error>((i, spv))
            }));
        }

        // Collect results
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let (idx, spv) = handle.await.context("task panicked")??;
            results.push((idx, spv));
        }
        results.sort_by_key(|(idx, _)| *idx);

        info!("All {} script groups executed in {:?}", script_count, start.elapsed());
        info!("Per-script results:");
        for (idx, spv) in &results {
            log_single_pv(*idx, spv);
        }

        // Build and save PublicValues for comparison with monolithic
        let script_results: Vec<_> = results
            .iter()
            .map(|(_, spv)| {
                ckb_zkvm_lib::ScriptExecutionResult::new(
                    spv.script_hash,
                    spv.script_group_type,
                    spv.exit_code,
                    spv.cycles,
                )
            })
            .collect();

        let pv = PublicValues::new_batched(tx_hash, script_results);
        let pv_json_path = args.output.join("public_values.json");
        let pv_json = serde_json::to_string_pretty(&pv)?;
        std::fs::write(&pv_json_path, &pv_json)?;
        info!("Public values saved to: {}", pv_json_path.display());
    } else {
        // Full proof generation with aggregation
        info!("Step 4: Setting up proving key for guest-single...");
        let single_elf_ref = Elf::Dynamic(single_elf.clone());
        let single_pk = client
            .setup(single_elf_ref)
            .await
            .context("SP1 setup failed for guest-single")?;
        let single_vk = single_pk.verifying_key().clone();
        let single_pk = Arc::new(single_pk);

        // Step 5: Parallel per-script proving
        info!(
            "Step 5: Generating {} compressed proofs in parallel (max {} concurrent)...",
            script_count, args.max_parallel
        );
        let start = Instant::now();

        let semaphore = Arc::new(Semaphore::new(args.max_parallel));
        let client = Arc::new(client);

        let mut handles = Vec::with_capacity(individual_traces.len());

        for (i, trace_bytes) in individual_traces.into_iter().enumerate() {
            let sem = semaphore.clone();
            let client = client.clone();
            let pk = single_pk.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");

                info!("  Starting proof for script group {}...", i);
                let prove_start = Instant::now();

                let mut stdin = SP1Stdin::new();
                stdin.write(&trace_bytes);

                let proof = client
                    .prove(&pk, stdin)
                    .compressed()
                    .await
                    .with_context(|| format!("proof generation failed for script group {}", i))?;

                info!(
                    "  Script group {} proved in {:?}",
                    i,
                    prove_start.elapsed()
                );

                Ok::<(usize, SP1ProofWithPublicValues), anyhow::Error>((i, proof))
            }));
        }

        // Collect proof results
        let mut proof_results: Vec<(usize, SP1ProofWithPublicValues)> =
            Vec::with_capacity(handles.len());
        for handle in handles {
            let result = handle.await.context("proving task panicked")??;
            proof_results.push(result);
        }
        proof_results.sort_by_key(|(idx, _)| *idx);

        info!(
            "All {} compressed proofs generated in {:?}",
            script_count,
            start.elapsed()
        );

        // Log per-script results
        info!("Per-script proof results:");
        for (idx, proof) in &proof_results {
            let spv = SingleScriptPublicValues::from_bytes(proof.public_values.as_slice())
                .ok_or_else(|| anyhow!("failed to decode public values for script group {}", idx))?;
            log_single_pv(*idx, &spv);
        }

        // Step 6: Setup aggregation proving key
        info!("Step 6: Setting up aggregation proving key...");
        let agg_elf = Elf::Dynamic(aggregation_elf.clone());

        let agg_pk = client
            .setup(agg_elf)
            .await
            .context("SP1 setup failed for guest-aggregation")?;

        // Step 7: Aggregation
        info!("Step 7: Aggregating {} proofs...", script_count);
        let start = Instant::now();

        // Collect vkeys (all the same since all use the same guest-single program)
        let vkeys: Vec<[u32; 8]> = proof_results
            .iter()
            .map(|_| single_vk.hash_u32())
            .collect();

        // Collect raw public values
        let all_public_values: Vec<Vec<u8>> = proof_results
            .iter()
            .map(|(_, proof)| proof.public_values.to_vec())
            .collect();

        // Build aggregation stdin
        let mut agg_stdin = SP1Stdin::new();
        agg_stdin.write(&vkeys);
        agg_stdin.write(&all_public_values);

        // Write each compressed proof for deferred verification
        for (idx, proof) in &proof_results {
            let SP1Proof::Compressed(ref inner) = proof.proof else {
                return Err(anyhow!(
                    "expected compressed proof for script group {}, got different type",
                    idx
                ));
            };
            agg_stdin.write_proof(*inner.clone(), single_vk.vk.clone());
        }

        let agg_proof = client
            .prove(&agg_pk, agg_stdin)
            .core()
            .await
            .context("aggregation proof generation failed")?;

        info!("Aggregation completed in {:?}", start.elapsed());

        // Step 8: Extract public values and save artifacts
        info!("Step 8: Saving artifacts...");

        let public_values = PublicValues::from_bytes(agg_proof.public_values.as_slice())
            .ok_or_else(|| anyhow!("failed to decode aggregated public values"))?;

        log_public_values(&public_values);

        let proof_path = args.output.join("proof.bin");
        let proof_bytes =
            bincode::serialize(&agg_proof).context("failed to serialize aggregated proof")?;
        std::fs::write(&proof_path, &proof_bytes).context("failed to write proof")?;
        info!("Proof saved to: {}", proof_path.display());

        let agg_vk = agg_pk.verifying_key().clone();
        let vk_path = args.output.join("vkey.bin");
        let vk_bytes =
            bincode::serialize(&agg_vk).context("failed to serialize verifying key")?;
        std::fs::write(&vk_path, &vk_bytes).context("failed to write verifying key")?;
        info!("Verifying key saved to: {}", vk_path.display());

        let pv_json_path = args.output.join("public_values.json");
        let pv_json = serde_json::to_string_pretty(&public_values)?;
        std::fs::write(&pv_json_path, &pv_json)?;
        info!("Public values saved to: {}", pv_json_path.display());

        info!("Run verify-parallel to verify the proof:");
    }

    info!("=============================================================");
    info!("Done!");

    Ok(())
}
