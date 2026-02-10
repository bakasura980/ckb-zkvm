//! CKB-zkVM Proof Verification CLI
//!
//! Verifies a zero-knowledge proof for a CKB transaction.
//!
//! Usage:
//!   verify <PROOF_PATH> [--vkey <PATH>] [--expected-hash <HASH>] [--elf <PATH>]

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1VerifyingKey};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use ckb_zkvm_lib::{parse_h256, PublicValues};

/// CLI arguments for the verify command.
#[derive(Parser, Debug)]
#[command(name = "verify")]
#[command(about = "Verify a ZK proof for a CKB transaction")]
struct Args {
    /// Path to the proof file
    #[arg(value_name = "PROOF")]
    proof_path: PathBuf,

    /// Path to the guest program ELF binary (required if vkey not provided)
    #[arg(long, env = "GUEST_ELF_PATH", default_value = "./target/elf-compilation/riscv32im-succinct-zkvm-elf/release/ckb-zkvm-program")]
    elf: PathBuf,

    /// Path to the verification key file (optional, will derive from ELF if not provided)
    #[arg(long, short)]
    vkey: Option<PathBuf>,

    /// Expected transaction hash (optional, for additional validation)
    #[arg(long)]
    expected_hash: Option<String>,
}

// Lyubo: Export that to a Readme file.
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

    info!("CKB-zkVM Proof Verifier");
    info!("=======================");
    info!("Proof: {}", args.proof_path.display());

    // Step 1: Load proof
    info!("Step 1: Loading proof...");
    let start = Instant::now();

    let proof_bytes =
        std::fs::read(&args.proof_path).context("failed to read proof file")?;

    let proof: SP1ProofWithPublicValues =
        bincode::deserialize(&proof_bytes).context("failed to deserialize proof")?;

    info!("Proof loaded in {:?}", start.elapsed());

    // Step 2: Extract public values
    info!("Step 2: Extracting public values...");

    let public_values_bytes = proof.public_values.as_slice();
    let public_values = PublicValues::from_bytes(public_values_bytes)
        .ok_or_else(|| anyhow!("failed to decode public values"))?;

    info!("Public values:");
    info!("  tx_hash: 0x{}", hex::encode(public_values.tx_hash));
    info!("  exit_code: {}", public_values.exit_code);
    info!("  cycles: {}", public_values.cycles);

    // Step 3: Validate expected hash if provided
    if let Some(expected) = &args.expected_hash {
        info!("Step 3: Validating transaction hash...");

        let expected_hash = parse_h256(expected).context("invalid expected hash")?;

        if public_values.tx_hash != expected_hash {
            return Err(anyhow!(
                "transaction hash mismatch: expected 0x{}, got 0x{}",
                hex::encode(expected_hash),
                hex::encode(public_values.tx_hash)
            ));
        }

        info!("Transaction hash matches expected value");
    }

    // Step 4: Load or derive verification key
    info!("Step 4: Loading verification key...");

    let vk: SP1VerifyingKey = if let Some(vkey_path) = &args.vkey {
        let vkey_bytes =
            std::fs::read(vkey_path).context("failed to read vkey file")?;
        bincode::deserialize(&vkey_bytes).context("failed to deserialize vkey")?
    } else {
        info!("No vkey provided, deriving from guest ELF at {}...", args.elf.display());
        let guest_elf = std::fs::read(&args.elf)
            .with_context(|| format!("failed to read guest ELF from {}", args.elf.display()))?;
        let client = ProverClient::from_env();
        let (_, vk) = client.setup(&guest_elf);
        vk
    };

    // Step 5: Verify proof
    info!("Step 5: Verifying proof...");
    let start = Instant::now();

    let client = ProverClient::from_env();
    client
        .verify(&proof, &vk)
        .context("proof verification failed")?;

    info!("Proof verified in {:?}", start.elapsed());

    // Step 6: Report result
    info!("=======================");

    if public_values.is_success() {
        info!("VALID: Proof confirms successful execution of transaction");
        info!("  Transaction: 0x{}", hex::encode(public_values.tx_hash));
        info!("  Cycles: {}", public_values.cycles);
    } else {
        info!(
            "VALID: Proof confirms execution with exit code {}",
            public_values.exit_code
        );
    }

    Ok(())
}
