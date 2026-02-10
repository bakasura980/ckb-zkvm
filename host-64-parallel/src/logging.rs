use ckb_zkvm_lib::{PublicValues, SingleScriptPublicValues};
use tracing::info;

pub fn log_public_values(pv: &PublicValues) {
    info!("Public values:");
    info!("  tx_hash: 0x{}", hex::encode(pv.tx_hash));
    info!("  exit_code: {}", pv.exit_code);
    info!("  cycles: {}", pv.cycles);
    info!("  script_results: {} groups", pv.script_results.len());
    for (i, sr) in pv.script_results.iter().enumerate() {
        let group_type = if sr.script_group_type == 0 {
            "Lock"
        } else {
            "Type"
        };
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

pub fn log_single_pv(index: usize, spv: &SingleScriptPublicValues) {
    let group_type = if spv.script_group_type == 0 {
        "Lock"
    } else {
        "Type"
    };
    info!(
        "  [{}] {} script_hash=0x{} exit_code={} cycles={}",
        index,
        group_type,
        hex::encode(spv.script_hash),
        spv.exit_code,
        spv.cycles
    );
}
