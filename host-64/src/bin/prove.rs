//! CKB-zkVM Proof Generation CLI (SP1 Hypercube v6 — 64-bit)
//!
//! Generates a zero-knowledge proof for a CKB transaction using
//! SP1 Hypercube with native 64-bit RISC-V support.
//!
//! Usage:
//!   prove-v6 <TX_HASH> [--rpc <URL>] [--output <PATH>] [--elf <PATH>]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use sp1_sdk::{Elf, ProveRequest, Prover, ProverClient, ProvingKey, SP1Stdin};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use ckb_zkvm_lib::{parse_h256, PublicValues};
use ckb_zkvm_host_64::executor::TransactionExecutor;
use ckb_zkvm_host_64::rpc::{CkbRpcClient, RpcConfig};

/// CLI arguments for the prove command.
#[derive(Parser, Debug)]
#[command(name = "prove-v6")]
#[command(about = "Generate a ZK proof for a CKB transaction (SP1 Hypercube v6, 64-bit)")]
struct Args {
    /// Transaction hash to prove (with or without 0x prefix)
    #[arg(value_name = "TX_HASH")]
    tx_hash: String,

    /// Path to the guest program ELF binary
    #[arg(long, env = "GUEST_ELF_PATH", default_value = env!("SP1_ELF_ckb-zkvm-program-64"))]
    elf: PathBuf,

    /// CKB RPC endpoint URL
    #[arg(long, env = "CKB_RPC_URL", default_value = "https://testnet.ckb.dev/rpc")]
    rpc: String,

    /// Output directory for proof artifacts
    #[arg(long, short, env = "OUTPUT_DIR", default_value = "./output")]
    output: PathBuf,

    /// Skip SP1 entirely (only run host-side trace collection)
    #[arg(long)]
    execute_only: bool,

    /// Run the SP1 guest program without generating a proof
    #[arg(long)]
    no_prove: bool,
}

fn log_public_values(pv: &PublicValues) {
    info!("Public values:");
    info!("  tx_hash: 0x{}", hex::encode(pv.tx_hash));
    info!("  exit_code: {}", pv.exit_code);
    info!("  cycles: {}", pv.cycles);
    info!("  script_results: {} groups", pv.script_results.len());
    for (i, sr) in pv.script_results.iter().enumerate() {
        let group_type = if sr.script_group_type == 0 { "Lock" } else { "Type" };
        info!(
            "    [{}] {} script_hash=0x{} exit_code={} cycles={}",
            i,
            group_type,
            hex::encode(sr.script_hash),
            sr.exit_code,
            sr.cycles
        );
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

    // Parse CLI arguments
    let args = Args::parse();

    info!("CKB-zkVM Proof Generator (SP1 Hypercube v6, 64-bit)");
    info!("=====================================================");
    info!("Transaction: {}", args.tx_hash);
    info!("RPC: {}", args.rpc);
    info!("ELF: {}", args.elf.display());

    // Load guest ELF from file
    let guest_elf: Arc<[u8]> = std::fs::read(&args.elf)
        .with_context(|| format!("failed to read guest ELF from {}", args.elf.display()))?
        .into();

    // Parse transaction hash
    let tx_hash = parse_h256(&args.tx_hash).context("invalid transaction hash")?;

    // Create output directory
    std::fs::create_dir_all(&args.output).context("failed to create output directory")?;

    // Step 1: Fetch transaction and generate trace
    info!("Step 1: Fetching transaction and generating trace...");
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

    info!(
        "Traces generated in {:?} ({} script groups, {} bytes total)",
        start.elapsed(),
        tx_traces.script_count(),
        tx_traces.encode_to_vec().len()
    );

    // Step 2: Serialize traces to SP1 input
    info!("Step 2: Preparing SP1 input...");
    let trace_bytes = tx_traces.encode_to_vec();

    let mut stdin = SP1Stdin::new();
    stdin.write(&trace_bytes);

    if args.execute_only {
        info!("Execute-only mode: skipping SP1 execution");
        return Ok(());
    }

    // Step 3: Run SP1 guest program (v6 async API)
    let client = ProverClient::from_env().await;

    if args.no_prove {
        // Execute the guest program without proof generation
        info!("Step 3: Executing SP1 guest program (no proof)...");
        let start = Instant::now();

        let elf = Elf::Dynamic(guest_elf.clone());
        let (public_values_raw, report) = client
            .execute(elf, stdin)
            .await
            .context("SP1 execution failed")?;

        info!("SP1 execution completed in {:?}", start.elapsed());
        info!("  SP1 cycles: {}", report.total_instruction_count());

        let public_values = PublicValues::from_bytes(public_values_raw.as_slice())
            .context("failed to decode public values")?;

        log_public_values(&public_values);

        // Save public values as JSON
        let pv_json_path = args.output.join("public_values.json");
        let pv_json = serde_json::to_string_pretty(&public_values)?;
        std::fs::write(&pv_json_path, &pv_json)?;
        info!("Public values saved to: {}", pv_json_path.display());
    } else {
        // Full proof generation
        let proving_start = Instant::now();
        let elf = Elf::Dynamic(guest_elf.clone());

        info!("Step 3: Setting up proving key...");
        let pk = client
            .setup(elf)
            .await
            .context("SP1 setup failed")?;
        let vk = pk.verifying_key().clone();

        info!("Step 4: Generating proof (this may take a while)...");
        let start = Instant::now();

        let proof = client
            .prove(&pk, stdin)
            .core()
            .await
            .context("proof generation failed")?;

        info!("Proof generated in {:?}", start.elapsed());

        // Extract public values
        info!("Step 5: Decoding public values...");

        let public_values_bytes = proof.public_values.as_slice();
        let public_values = PublicValues::from_bytes(public_values_bytes)
            .context("failed to decode public values")?;

        log_public_values(&public_values);

        // Verify proof
        info!("Step 6: Verifying proof...");
        let start = Instant::now();

        client
            .verify(&proof, &vk, None)
            .context("proof verification failed")?;

        info!("Proof verified in {:?}", start.elapsed());

        info!("Total proving time: {:?} (setup + proof + verify)", proving_start.elapsed());

        // Save artifacts
        info!("Step 7: Saving artifacts...");

        let proof_path = args.output.join("proof.bin");
        let proof_bytes = bincode::serialize(&proof).context("failed to serialize proof")?;
        std::fs::write(&proof_path, &proof_bytes).context("failed to write proof")?;
        info!("Proof saved to: {}", proof_path.display());

        let vkey_path = args.output.join("vkey.bin");
        let vkey_bytes = bincode::serialize(&vk).context("failed to serialize vkey")?;
        std::fs::write(&vkey_path, &vkey_bytes).context("failed to write vkey")?;
        info!("Verifying key saved to: {}", vkey_path.display());

        let pv_json_path = args.output.join("public_values.json");
        let pv_json = serde_json::to_string_pretty(&public_values)?;
        std::fs::write(&pv_json_path, &pv_json)?;
        info!("Public values saved to: {}", pv_json_path.display());
    }

    info!("=====================================================");
    info!("Done!");

    Ok(())
}
