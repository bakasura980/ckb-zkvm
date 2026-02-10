//! CKB-zkVM Verification Key Extraction CLI
//!
//! Extracts the verification key from the guest ELF for external use.
//!
//! Usage:
//!   vkey [--elf <PATH>] [--output <PATH>] [--format <FORMAT>]

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use sp1_sdk::{HashableKey, ProverClient};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Output format for the verification key.
#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    /// Binary format (bincode serialization)
    Bin,
    /// Hex-encoded string
    Hex,
    /// JSON format with hash and encoded data
    Json,
}

/// CLI arguments for the vkey command.
#[derive(Parser, Debug)]
#[command(name = "vkey")]
#[command(about = "Extract verification key from guest ELF")]
struct Args {
    /// Path to the guest program ELF binary
    #[arg(long, env = "GUEST_ELF_PATH", default_value = "./target/elf-compilation/riscv32im-succinct-zkvm-elf/release/ckb-zkvm-program")]
    elf: PathBuf,

    /// Output file path
    #[arg(long, short, default_value = "./output/vkey.bin")]
    output: PathBuf,

    /// Output format
    #[arg(long, short, value_enum, default_value = "bin")]
    format: OutputFormat,

    /// Print vkey hash to stdout
    #[arg(long)]
    print_hash: bool,
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

    info!("CKB-zkVM Verification Key Extractor");
    info!("====================================");

    // Create output directory if needed
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent).context("failed to create output directory")?;
    }

    // Load guest ELF and extract vkey
    info!("Loading guest ELF from {}...", args.elf.display());
    let guest_elf = std::fs::read(&args.elf)
        .with_context(|| format!("failed to read guest ELF from {}", args.elf.display()))?;

    info!("Deriving verification key from guest ELF...");
    let client = ProverClient::from_env();
    let (_, vk) = client.setup(&guest_elf);

    // Get vkey hash
    let vk_hash = vk.bytes32();
    info!("Verification key hash: {}", vk_hash);

    if args.print_hash {
        println!("{}", vk_hash);
    }

    // Serialize based on format
    let output_data = match args.format {
        OutputFormat::Bin => {
            let data = bincode::serialize(&vk).context("failed to serialize vkey")?;
            info!("Binary format: {} bytes", data.len());
            data
        }
        OutputFormat::Hex => {
            let data = bincode::serialize(&vk).context("failed to serialize vkey")?;
            let hex_str = hex::encode(&data);
            info!("Hex format: {} characters", hex_str.len());
            hex_str.into_bytes()
        }
        OutputFormat::Json => {
            let data = bincode::serialize(&vk).context("failed to serialize vkey")?;
            let json = serde_json::json!({
                "hash": vk_hash,
                "encoded": hex::encode(&data),
            });
            let json_str =
                serde_json::to_string_pretty(&json).context("failed to serialize JSON")?;
            info!("JSON format: {} bytes", json_str.len());
            json_str.into_bytes()
        }
    };

    // Write output
    std::fs::write(&args.output, &output_data).context("failed to write output file")?;
    info!("Verification key saved to: {}", args.output.display());

    info!("====================================");
    info!("Done!");

    Ok(())
}
