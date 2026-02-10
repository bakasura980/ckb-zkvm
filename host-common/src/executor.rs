//! Transaction executor for generating execution traces.
//!
//! This module orchestrates:
//! 1. Fetching transaction data from CKB RPC
//! 2. Building a mock transaction for local execution
//! 3. Identifying all script groups (lock and type scripts)
//! 4. Executing with collectors (two-phase pattern) for each script group
//! 5. Producing a TransactionTraces containing all script traces
//!
//! ## Script Groups
//!
//! A CKB transaction executes multiple scripts:
//! - Lock scripts: One for each input cell (authorizes spending)
//! - Type scripts: For each input/output with a type script
//!
//! Scripts are grouped by (code_hash, hash_type, args) hash into "script groups".
//! Each group executes once, even if multiple cells use the same script.

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use ckb_jsonrpc_types::{CellWithStatus, Either, OutPoint, Transaction, TransactionWithStatusResponse};
use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction};
use ckb_script::ScriptGroupType;
use ckb_types::{
    bytes::Bytes,
    core::{DepType, HeaderView as CoreHeaderView, ScriptHashType, TransactionBuilder, TransactionView as CoreTransactionView},
    packed::{Byte32, Byte, CellDep, CellInput, CellOutput, OutPoint as PackedOutPoint, Script, Uint32, Uint64},
    prelude::*,
};
use ckb_vm_contrib_collectors::{Collector, SyscallBasedCollector};
use ckb_zkvm_lib::{ExecutionTrace, TransactionTraces, VmCreation, H256, ELF_MAGIC};
use indexmap::IndexMap;
use tracing::{debug, info, instrument};

use crate::rpc::CkbRpcClient;

/// Script group type constants.
pub const SCRIPT_GROUP_LOCK: u32 = 0;
pub const SCRIPT_GROUP_TYPE: u32 = 1;

/// CKB's built-in Type ID code hash (ASCII "TYPE_ID" zero-padded to 32 bytes).
/// This script is implemented natively in the CKB node, not as a RISC-V binary.
const TYPE_ID_CODE_HASH: H256 = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x54, 0x59, 0x50, 0x45, 0x5f, 0x49, 0x44,
];

/// Transaction executor for trace generation.
pub struct TransactionExecutor {
    rpc: CkbRpcClient,
}

/// Data for a cell including its out_point.
#[derive(Debug, Clone)]
pub struct CellData {
    pub out_point: OutPoint,
    pub cell: CellWithStatus,
}

/// Collected transaction data needed for execution.
#[derive(Debug)]
pub struct TransactionData {
    pub tx_hash: H256,
    pub tx: Transaction,
    pub input_cells: Vec<CellData>,
    /// Block hash where each input cell's parent tx was confirmed.
    /// Parallel to `input_cells`. Needed for scripts that call
    /// `ckb_load_header(Source::Input, ...)` (e.g. DAO withdrawals).
    pub input_block_hashes: Vec<ckb_types::H256>,
    /// Cell deps parallel to tx.cell_deps
    pub dep_cells: Vec<CellData>,
    /// Additional cells from DepGroup expansion
    pub expanded_dep_cells: Vec<CellData>,
    /// Block headers referenced by header_deps
    pub header_deps: Vec<ckb_jsonrpc_types::HeaderView>,
}

/// Information about a script group to be executed.
#[derive(Debug, Clone)]
pub struct ScriptGroup {
    /// Script hash (hash of the script itself)
    pub script_hash: H256,
    /// Code hash from the script
    pub code_hash: H256,
    /// Hash type (Data, Type, Data1, Data2)
    pub hash_type: ScriptHashType,
    /// Script group type (Lock = 0, Type = 1)
    pub group_type: u32,
}

impl TransactionExecutor {
    /// Create a new transaction executor.
    pub fn new(rpc: CkbRpcClient) -> Self {
        Self { rpc }
    }

    /// Execute a transaction and generate traces for ALL script groups.
    /// Returns a TransactionTraces containing one ExecutionTrace per script group.
    #[instrument(skip(self))]
    pub async fn execute_all(&self, tx_hash: H256) -> Result<TransactionTraces> {
        let tx_hash_hex = format!("0x{}", hex::encode(tx_hash));
        info!("executing all scripts for transaction {}", tx_hash_hex);

        // Step 1: Fetch all transaction data
        let tx_data = self.fetch_transaction_data(tx_hash).await?;

        // Step 2: Identify all script groups
        let script_groups = self.identify_script_groups(&tx_data)?;
        info!("identified {} script groups", script_groups.len());

        // Step 3: Build MockTransaction and ELF cache once (shared across all script groups)
        let mock_tx = self.build_mock_transaction(&tx_data)?;
        let elf_cache = self.build_elf_cache(&tx_data)?;

        // Step 4: Collect trace for each script group using collector pattern
        let mut tx_traces = TransactionTraces::new(tx_hash);

        for group in &script_groups {
            // Skip Type ID — it's a built-in CKB node script with no on-chain ELF binary.
            if group.code_hash == TYPE_ID_CODE_HASH {
                info!(
                    "skipping Type ID script group {} (built-in, no ELF)",
                    hex::encode(group.script_hash)
                );
                continue;
            }

            debug!(
                "collecting trace for script group: {} (type={})",
                hex::encode(group.script_hash),
                if group.group_type == SCRIPT_GROUP_LOCK { "Lock" } else { "Type" }
            );

            let trace = self.collect_script_trace(&tx_data, &mock_tx, &elf_cache, group)?;
            tx_traces.add_script_trace(trace);
        }

        info!(
            "generated {} script traces for transaction",
            tx_traces.script_count()
        );

        Ok(tx_traces)
    }

    /// Identify all script groups in the transaction.
    fn identify_script_groups(&self, tx_data: &TransactionData) -> Result<Vec<ScriptGroup>> {
        let mut groups: HashMap<H256, ScriptGroup> = HashMap::new();

        // Collect lock scripts from input cells
        for (i, cell_data) in tx_data.input_cells.iter().enumerate() {
            let cell_info = cell_data.cell
                .cell
                .as_ref()
                .ok_or_else(|| anyhow!("input cell {} not found", i))?;

            let lock = &cell_info.output.lock;
            let script_hash = self.compute_script_hash(lock)?;
            let code_hash: [u8; 32] = lock.code_hash.clone().into();

            groups.entry(script_hash).or_insert_with(|| ScriptGroup {
                script_hash,
                code_hash,
                hash_type: self.convert_hash_type(&lock.hash_type),
                group_type: SCRIPT_GROUP_LOCK,
            });
        }

        // Collect type scripts from input cells
        for (i, cell_data) in tx_data.input_cells.iter().enumerate() {
            let cell_info = cell_data.cell
                .cell
                .as_ref()
                .ok_or_else(|| anyhow!("input cell {} not found", i))?;

            if let Some(type_script) = &cell_info.output.type_ {
                let script_hash = self.compute_script_hash(type_script)?;
                let code_hash: [u8; 32] = type_script.code_hash.clone().into();

                groups.entry(script_hash).or_insert_with(|| ScriptGroup {
                    script_hash,
                    code_hash,
                    hash_type: self.convert_hash_type(&type_script.hash_type),
                    group_type: SCRIPT_GROUP_TYPE,
                });
            }
        }

        // Collect type scripts from output cells
        for output in &tx_data.tx.outputs {
            if let Some(type_script) = &output.type_ {
                let script_hash = self.compute_script_hash(type_script)?;
                let code_hash: [u8; 32] = type_script.code_hash.clone().into();

                groups.entry(script_hash).or_insert_with(|| ScriptGroup {
                    script_hash,
                    code_hash,
                    hash_type: self.convert_hash_type(&type_script.hash_type),
                    group_type: SCRIPT_GROUP_TYPE,
                });
            }
        }

        // Convert to Vec, sorted for deterministic ordering
        let mut result: Vec<ScriptGroup> = groups.into_values().collect();
        result.sort_unstable_by(|a, b| {
            // Sort by group type first (Lock before Type), then by script hash
            a.group_type.cmp(&b.group_type)
                .then_with(|| a.script_hash.cmp(&b.script_hash))
        });

        Ok(result)
    }

    /// Compute the script hash for a JSON-RPC script.
    fn compute_script_hash(&self, script: &ckb_jsonrpc_types::Script) -> Result<H256> {
        let packed_script = self.convert_script(script)?;
        let hash = packed_script.calc_script_hash();
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_slice());
        Ok(result)
    }

    /// Convert JSON-RPC hash type to core hash type.
    fn convert_hash_type(&self, hash_type: &ckb_jsonrpc_types::ScriptHashType) -> ScriptHashType {
        match hash_type {
            ckb_jsonrpc_types::ScriptHashType::Data => ScriptHashType::Data,
            ckb_jsonrpc_types::ScriptHashType::Type => ScriptHashType::Type,
            ckb_jsonrpc_types::ScriptHashType::Data1 => ScriptHashType::Data1,
            ckb_jsonrpc_types::ScriptHashType::Data2 => ScriptHashType::Data2,
            _ => ScriptHashType::Data2,
        }
    }

    /// Collect trace for a specific script group.
    fn collect_script_trace(
        &self,
        tx_data: &TransactionData,
        mock_tx: &MockTransaction,
        elf_cache: &HashMap<H256, Vec<u8>>,
        group: &ScriptGroup,
    ) -> Result<ExecutionTrace> {
        // Find the ELF binary for this script's code_hash
        let elf = self.find_script_binary(elf_cache, &group.code_hash)?;

        // Create the execution trace (ELF stored once in trace.elf)
        let mut trace = ExecutionTrace::new(tx_data.tx_hash, elf, Vec::new());
        trace.set_script_hash(group.script_hash);
        trace.set_script_group_type(group.group_type);

        // Execute and collect syscalls for this script group using the collector pattern
        let syscall_trace = self.execute_and_collect_for_group(mock_tx, group)?;
        trace.set_syscall_trace(syscall_trace);

        // Record root VM creation (no binary locator — guest uses trace.elf directly)
        let root_vm = VmCreation::new(0, 0, None, None);
        trace.add_vm_creation(root_vm);

        Ok(trace)
    }

    /// Build a cache of ELF binaries keyed by code_hash.
    ///
    /// Scans all dep cells once, computing data hashes and type script hashes
    /// for each ELF binary. Returns a map from code_hash to ELF bytes.
    /// This avoids repeated linear scans, clones, and hash recomputations
    /// when multiple script groups share the same code_hash.
    fn build_elf_cache(&self, tx_data: &TransactionData) -> Result<HashMap<H256, Vec<u8>>> {
        let mut cache: HashMap<H256, Vec<u8>> = HashMap::new();

        let all_deps = tx_data.dep_cells.iter()
            .chain(tx_data.expanded_dep_cells.iter());

        for cell_data in all_deps {
            let cell = match &cell_data.cell.cell {
                Some(c) => c,
                None => continue,
            };
            let data = match &cell.data {
                Some(d) => d,
                None => continue,
            };

            let bytes: Vec<u8> = data.content.as_bytes().to_vec();

            // Check ELF magic
            if bytes.len() < 4 || bytes[0..4] != ELF_MAGIC {
                continue;
            }

            // Compute data hash (for Data/Data1/Data2 hash types)
            let data_hash = CellOutput::calc_data_hash(&bytes);
            let mut data_hash_bytes = [0u8; 32];
            data_hash_bytes.copy_from_slice(data_hash.as_slice());

            // Compute type script hash if present (for Type hash type)
            let type_hash = if let Some(type_script) = &cell.output.type_ {
                Some(self.compute_script_hash(type_script)?)
            } else {
                None
            };

            // Insert by type hash first (avoids clone when both keys needed)
            if let Some(th) = type_hash {
                cache.entry(th).or_insert_with(|| bytes.clone());
            }
            // Insert by data hash
            cache.entry(data_hash_bytes).or_insert(bytes);
        }

        debug!("built ELF cache with {} entries", cache.len());
        Ok(cache)
    }

    /// Look up an ELF binary by code_hash from the pre-built cache.
    fn find_script_binary(
        &self,
        elf_cache: &HashMap<H256, Vec<u8>>,
        code_hash: &H256,
    ) -> Result<Vec<u8>> {
        elf_cache
            .get(code_hash)
            .cloned()
            .ok_or_else(|| anyhow!(
                "no ELF binary found for code_hash {} in ELF cache ({} entries)",
                hex::encode(code_hash),
                elf_cache.len()
            ))
    }

    /// Execute and collect syscalls for a specific script group using the collector pattern.
    ///
    /// This uses SyscallBasedCollector from ckb-vm-contrib to record all syscall
    /// responses during VM execution. The recorded trace can then be replayed
    /// in the zkVM guest program.
    fn execute_and_collect_for_group(
        &self,
        mock_tx: &MockTransaction,
        group: &ScriptGroup,
    ) -> Result<Vec<u8>> {
        // Create the syscall-based collector
        let collector: SyscallBasedCollector = SyscallBasedCollector::default();

        // Build verifier from the mock transaction using the collector pattern
        let verifier = collector.build_verifier(&mock_tx)
            .map_err(|e| anyhow!("failed to build verifier: {:?}", e))?;

        // Determine script group type for lookup
        let group_type = if group.group_type == SCRIPT_GROUP_LOCK {
            ScriptGroupType::Lock
        } else {
            ScriptGroupType::Type
        };

        // Find the script group in the verifier
        let script_hash_packed = Byte32::new(group.script_hash);
        let script_group = verifier
            .find_script_group(group_type, &script_hash_packed)
            .ok_or_else(|| anyhow!(
                "script group not found for hash {} (type={:?})",
                hex::encode(group.script_hash),
                group_type
            ))?;

        // Execute and collect syscalls
        info!(
            "collecting syscalls for script group {} ({:?})",
            hex::encode(group.script_hash),
            group_type
        );

        let result = collector.collect(&verifier, script_group)
            .map_err(|e| anyhow!("failed to collect syscalls: {:?}", e))?;

        info!(
            "script group {} completed with exit_code={}, cycles={}",
            hex::encode(group.script_hash),
            result.exit_code,
            result.cycles
        );

        // Extract the root VM's gen 0 trace (vm_id=0, generation_id=0).
        //
        // The collector returns traces keyed by (vm_id, generation_id):
        // - Simple scripts: single entry at (0, 0)
        // - Scripts using `exec`: multiple generations for vm_id=0, but the guest's
        //   ExecHaltingSyscalls halts the VM on exec (syscall 2043) so only gen 0's
        //   syscalls are ever consumed. Gen 1+ entries are discarded.
        // - Scripts using `spawn`: child VMs at vm_id>0 are discarded — the parent's
        //   spawn syscall response already contains the child's exit code.
        let gen0_trace = result.traces.iter()
            .find(|(k, _)| k.vm_id == 0 && k.generation_id == 0)
            .map(|(_, trace)| trace)
            .ok_or_else(|| anyhow!("no syscall trace found for root VM (vm_id=0, gen=0)"))?;

        let extra_gens = result.traces.keys()
            .filter(|k| k.vm_id == 0 && k.generation_id > 0)
            .count();
        if extra_gens > 0 {
            debug!(
                "script group {} has {} exec generation(s) beyond gen 0 (discarded)",
                hex::encode(group.script_hash),
                extra_gens
            );
        }

        let child_vm_count = result.traces.keys()
            .filter(|k| k.vm_id != 0)
            .count();
        if child_vm_count > 0 {
            debug!(
                "script group {} has {} child VM traces (spawn; discarded)",
                hex::encode(group.script_hash),
                child_vm_count
            );
        }

        let trace_bytes: Vec<u8> = gen0_trace.clone().into();
        Ok(trace_bytes)
    }

    /// Build a MockTransaction from TransactionData for use with collectors.
    fn build_mock_transaction(&self, tx_data: &TransactionData) -> Result<MockTransaction> {
        // Build mock inputs
        let mut mock_inputs = Vec::with_capacity(tx_data.input_cells.len());
        for (i, (input, cell_data)) in tx_data.tx.inputs.iter()
            .zip(tx_data.input_cells.iter())
            .enumerate()
        {
            let cell_info = cell_data.cell.cell.as_ref()
                .ok_or_else(|| anyhow!("input cell {} not found", i))?;

            // Convert input
            let cell_input = self.convert_cell_input(input)?;

            // Convert output
            let (output, _) = self.convert_cell(&cell_info.output)?;

            // Get cell data
            let data = if let Some(cd) = &cell_info.data {
                Bytes::copy_from_slice(cd.content.as_bytes())
            } else {
                Bytes::default()
            };

            mock_inputs.push(MockInput {
                input: cell_input,
                output,
                data,
                header: Some(Self::h256_to_byte32(&tx_data.input_block_hashes[i])),
            });
        }

        // Build mock cell deps (parallel to tx.cell_deps)
        let mut mock_cell_deps = Vec::with_capacity(
            tx_data.dep_cells.len() + tx_data.expanded_dep_cells.len()
        );

        for (i, (dep, cell_data)) in tx_data.tx.cell_deps.iter()
            .zip(tx_data.dep_cells.iter())
            .enumerate()
        {
            let cell_info = cell_data.cell.cell.as_ref()
                .ok_or_else(|| anyhow!("dep cell {} not found", i))?;

            // Convert cell dep
            let cell_dep = self.convert_cell_dep(dep)?;

            // Convert output
            let (output, _) = self.convert_cell(&cell_info.output)?;

            // Get cell data
            let data = if let Some(cd) = &cell_info.data {
                Bytes::copy_from_slice(cd.content.as_bytes())
            } else {
                Bytes::default()
            };

            mock_cell_deps.push(MockCellDep {
                cell_dep,
                output,
                data,
                header: None,
            });
        }

        // Add expanded dep cells (from DepGroup expansion) as Code type
        for (i, cell_data) in tx_data.expanded_dep_cells.iter().enumerate() {
            let cell_info = cell_data.cell.cell.as_ref()
                .ok_or_else(|| anyhow!("expanded dep cell {} not found", i))?;

            // Create CellDep with Code type for expanded cells
            let out_point = self.convert_out_point(&cell_data.out_point)?;
            let cell_dep = CellDep::new_builder()
                .out_point(out_point)
                .dep_type(Byte::new(DepType::Code.into()))
                .build();

            // Convert output
            let (output, _) = self.convert_cell(&cell_info.output)?;

            // Get cell data
            let data = if let Some(cd) = &cell_info.data {
                Bytes::copy_from_slice(cd.content.as_bytes())
            } else {
                Bytes::default()
            };

            mock_cell_deps.push(MockCellDep {
                cell_dep,
                output,
                data,
                header: None,
            });
        }

        // Convert header deps from JSON-RPC HeaderView to core HeaderView
        let mock_header_deps: Vec<CoreHeaderView> = tx_data
            .header_deps
            .iter()
            .map(|json_header| {
                let hash = json_header.hash.pack();
                CoreHeaderView::from(json_header.clone()).fake_hash(hash)
            })
            .collect();

        // Build the packed transaction
        let tx = self.build_core_transaction(tx_data)?.data();

        Ok(MockTransaction {
            mock_info: MockInfo {
                inputs: mock_inputs,
                cell_deps: mock_cell_deps,
                header_deps: mock_header_deps,
                extensions: Vec::new(),
            },
            tx,
        })
    }

    /// Convert JSON-RPC cell input to packed format.
    fn convert_cell_input(&self, input: &ckb_jsonrpc_types::CellInput) -> Result<CellInput> {
        let out_point = self.convert_out_point(&input.previous_output)?;
        let since: u64 = input.since.into();

        Ok(CellInput::new_builder()
            .previous_output(out_point)
            .since(Uint64::from_slice(&since.to_le_bytes()).expect("valid u64"))
            .build())
    }

    /// Convert JSON-RPC cell dep to packed format.
    fn convert_cell_dep(&self, dep: &ckb_jsonrpc_types::CellDep) -> Result<CellDep> {
        let out_point = self.convert_out_point(&dep.out_point)?;
        let dep_type = match dep.dep_type {
            ckb_jsonrpc_types::DepType::Code => DepType::Code,
            ckb_jsonrpc_types::DepType::DepGroup => DepType::DepGroup,
        };
        let dep_type_byte: Byte = dep_type.into();

        Ok(CellDep::new_builder()
            .out_point(out_point)
            .dep_type(dep_type_byte)
            .build())
    }

    /// Fetch transaction and all related cell data.
    ///
    /// Optimized with three layers:
    /// 1. Skip `get_live_cell` for inputs (always spent in confirmed txs)
    /// 2. Deduplicate parent transactions (multiple inputs may share a parent)
    /// 3. JSON-RPC batch requests (single HTTP POST for all fetches)
    #[instrument(skip(self))]
    async fn fetch_transaction_data(&self, tx_hash: H256) -> Result<TransactionData> {
        let tx_hash_hex = format!("0x{}", hex::encode(tx_hash));

        // Phase 1: Fetch the target transaction
        let tx_response = self
            .rpc
            .get_transaction(&tx_hash_hex)
            .await
            .context("failed to fetch transaction")?;

        let tx_either = tx_response
            .transaction
            .ok_or_else(|| anyhow!("transaction not found: {}", tx_hash_hex))?;

        let tx_view = match tx_either.inner {
            Either::Left(view) => view,
            Either::Right(_bytes) => {
                return Err(anyhow!("transaction returned as raw bytes, expected TransactionView"));
            }
        };

        let tx = tx_view.inner;
        debug!("fetched transaction with {} inputs, {} deps", tx.inputs.len(), tx.cell_deps.len());

        // Phase 2: Deduplicate parent tx_hashes from inputs
        // Map: parent_tx_hash -> [(input_index, output_index)]
        let mut parent_tx_map: IndexMap<ckb_types::H256, Vec<(usize, u32)>> = IndexMap::new();
        for (i, input) in tx.inputs.iter().enumerate() {
            let parent_hash = input.previous_output.tx_hash.clone();
            let output_idx: u32 = input.previous_output.index.into();
            parent_tx_map
                .entry(parent_hash)
                .or_default()
                .push((i, output_idx));
        }

        let unique_parent_hashes: Vec<String> = parent_tx_map
            .keys()
            .map(|h| format!("0x{}", hex::encode(h)))
            .collect();

        info!(
            "{} inputs deduplicated to {} unique parent transactions",
            tx.inputs.len(),
            unique_parent_hashes.len()
        );

        // Phase 3: Batch-fetch parent transactions + dep cells + headers in parallel
        let dep_out_points: Vec<OutPoint> = tx
            .cell_deps
            .iter()
            .map(|d| d.out_point.clone())
            .collect();

        let header_dep_hashes: Vec<String> = tx
            .header_deps
            .iter()
            .map(|h| format!("0x{}", hex::encode(h)))
            .collect();

        let (parent_tx_results, dep_cell_results, header_results) = tokio::join!(
            self.rpc.batch_get_transactions(&unique_parent_hashes),
            self.rpc.batch_get_live_cells(&dep_out_points, true),
            self.rpc.batch_get_headers(&header_dep_hashes),
        );

        let parent_txs = parent_tx_results.context("failed to batch-fetch parent transactions")?;
        let dep_cells_raw = dep_cell_results.context("failed to batch-fetch dep cells")?;
        let header_deps = header_results.context("failed to batch-fetch header deps")?;

        if !header_deps.is_empty() {
            debug!("fetched {} header deps", header_deps.len());
        }

        // Phase 4: Extract input cells from parent transactions
        let (input_cells, input_block_hashes) = self.extract_input_cells_from_parents(
            &tx,
            &parent_tx_map,
            &unique_parent_hashes,
            &parent_txs,
        )?;

        // Resolve dep cells, falling back to parent tx lookup for non-live cells
        let dep_cells = self
            .resolve_cells_with_fallback(&dep_out_points, dep_cells_raw, "dep")
            .await?;

        debug!(
            "fetched {} input cells + {} dep cells via batch RPC",
            input_cells.len(),
            dep_cells.len()
        );

        // Phase 5: Expand DepGroups and batch-fetch expanded cells
        let expanded_dep_cells = self.expand_all_dep_groups(&tx, &dep_cells).await?;

        debug!("expanded {} dep group cells", expanded_dep_cells.len());

        Ok(TransactionData {
            tx_hash,
            tx,
            input_cells,
            input_block_hashes,
            dep_cells,
            expanded_dep_cells,
            header_deps,
        })
    }

    /// Extract input cells from batch-fetched parent transactions.
    ///
    /// For each input, looks up the parent transaction by its tx_hash and
    /// extracts the output at the referenced index.
    fn extract_input_cells_from_parents(
        &self,
        tx: &Transaction,
        parent_tx_map: &IndexMap<ckb_types::H256, Vec<(usize, u32)>>,
        unique_parent_hashes: &[String],
        parent_txs: &[TransactionWithStatusResponse],
    ) -> Result<(Vec<CellData>, Vec<ckb_types::H256>)> {
        let mut input_cells: Vec<Option<(CellData, ckb_types::H256)>> = vec![None; tx.inputs.len()];

        for (map_idx, (parent_hash, entries)) in parent_tx_map.iter().enumerate() {
            let parent_response = &parent_txs[map_idx];
            let parent_tx_either = parent_response
                .transaction
                .as_ref()
                .ok_or_else(|| {
                    anyhow!(
                        "parent transaction not found: {}",
                        unique_parent_hashes[map_idx]
                    )
                })?;

            let parent_tx_view = match &parent_tx_either.inner {
                Either::Left(view) => view,
                Either::Right(_) => {
                    return Err(anyhow!(
                        "parent tx {} returned as raw bytes",
                        unique_parent_hashes[map_idx]
                    ));
                }
            };

            let parent_tx = &parent_tx_view.inner;

            for &(input_idx, output_idx) in entries {
                let output = parent_tx
                    .outputs
                    .get(output_idx as usize)
                    .ok_or_else(|| {
                        anyhow!(
                            "input {}: output index {} not found in parent tx {} (has {} outputs)",
                            input_idx,
                            output_idx,
                            unique_parent_hashes[map_idx],
                            parent_tx.outputs.len()
                        )
                    })?;

                let output_data = parent_tx
                    .outputs_data
                    .get(output_idx as usize)
                    .cloned()
                    .unwrap_or_default();

                let cell_info = ckb_jsonrpc_types::CellInfo {
                    output: output.clone(),
                    data: Some(ckb_jsonrpc_types::CellData {
                        content: output_data,
                        hash: ckb_types::H256::default(),
                    }),
                };

                let out_point = OutPoint {
                    tx_hash: parent_hash.clone(),
                    index: ckb_jsonrpc_types::Uint32::from(output_idx),
                };

                // Extract block_hash from the parent transaction's tx_status.
                // This is needed for scripts that call ckb_load_header(Source::Input, ...).
                let block_hash = parent_response.tx_status.block_hash.clone()
                    .ok_or_else(|| anyhow!(
                        "input {}: parent tx {} has no block_hash (not confirmed?)",
                        input_idx,
                        unique_parent_hashes[map_idx]
                    ))?;

                input_cells[input_idx] = Some((
                    CellData {
                        out_point,
                        cell: CellWithStatus {
                            cell: Some(cell_info),
                            status: "live".to_string(),
                        },
                    },
                    block_hash,
                ));
            }
        }

        // Verify all input cells were resolved and split into parallel vectors
        let resolved: Vec<(CellData, ckb_types::H256)> = input_cells
            .into_iter()
            .enumerate()
            .map(|(i, cell)| cell.ok_or_else(|| anyhow!("input cell {} was not resolved", i)))
            .collect::<Result<Vec<_>>>()?;

        let (cells, block_hashes): (Vec<_>, Vec<_>) = resolved.into_iter().unzip();
        Ok((cells, block_hashes))
    }

    /// Resolve live cell batch results, fetching non-live cells from their parent transactions.
    ///
    /// Takes batch `get_live_cell` results and their corresponding OutPoints.
    /// Any cells not found as live are fetched via `batch_get_transactions`
    /// using their parent tx_hash, then extracted by output index.
    async fn resolve_cells_with_fallback(
        &self,
        out_points: &[OutPoint],
        batch_results: Vec<CellWithStatus>,
        label: &str,
    ) -> Result<Vec<CellData>> {
        let mut cells = Vec::with_capacity(batch_results.len());
        let mut missing_indices = Vec::new();

        for (i, cell_status) in batch_results.into_iter().enumerate() {
            if cell_status.cell.is_none() {
                missing_indices.push(i);
            }
            cells.push(CellData {
                out_point: out_points[i].clone(),
                cell: cell_status,
            });
        }

        if missing_indices.is_empty() {
            return Ok(cells);
        }

        debug!(
            "fetching {} non-live {} cells from parent transactions",
            missing_indices.len(),
            label
        );

        // Dedup tx hashes — multiple missing cells may share the same parent tx
        let mut unique_tx_hashes: Vec<String> = missing_indices
            .iter()
            .map(|&i| format!("0x{}", hex::encode(&out_points[i].tx_hash)))
            .collect();
        unique_tx_hashes.sort();
        unique_tx_hashes.dedup();

        let parent_txs = self
            .rpc
            .batch_get_transactions(&unique_tx_hashes)
            .await
            .with_context(|| format!("failed to batch-fetch {} parent transactions", label))?;

        // Index fetched transactions by hash for O(1) lookup
        let tx_by_hash: std::collections::HashMap<&str, &TransactionWithStatusResponse> =
            unique_tx_hashes
                .iter()
                .zip(parent_txs.iter())
                .map(|(hash, tx)| (hash.as_str(), tx))
                .collect();

        for &cell_idx in &missing_indices {
            let tx_hash = format!("0x{}", hex::encode(&out_points[cell_idx].tx_hash));
            let tx_response = tx_by_hash.get(tx_hash.as_str()).ok_or_else(|| {
                anyhow!("{} cell {}: parent tx {} not in batch response", label, cell_idx, tx_hash)
            })?;
            let cell_data = self.extract_cell_from_tx_response(
                tx_response,
                &out_points[cell_idx],
                &format!("{} cell {}", label, cell_idx),
            )?;
            cells[cell_idx] = cell_data;
        }

        Ok(cells)
    }

    /// Extract a single cell from a transaction response by output index.
    fn extract_cell_from_tx_response(
        &self,
        tx_response: &TransactionWithStatusResponse,
        out_point: &OutPoint,
        label: &str,
    ) -> Result<CellData> {
        let tx_either = tx_response
            .transaction
            .as_ref()
            .ok_or_else(|| anyhow!("{}: parent tx not found", label))?;

        let tx_view = match &tx_either.inner {
            Either::Left(view) => view,
            Either::Right(_) => {
                return Err(anyhow!("{}: parent tx returned as raw bytes", label));
            }
        };

        let tx = &tx_view.inner;
        let output_idx: u32 = out_point.index.into();

        let output = tx
            .outputs
            .get(output_idx as usize)
            .ok_or_else(|| {
                anyhow!(
                    "{}: output index {} not found (tx has {} outputs)",
                    label,
                    output_idx,
                    tx.outputs.len()
                )
            })?;

        let output_data = tx
            .outputs_data
            .get(output_idx as usize)
            .cloned()
            .unwrap_or_default();

        let cell_info = ckb_jsonrpc_types::CellInfo {
            output: output.clone(),
            data: Some(ckb_jsonrpc_types::CellData {
                content: output_data,
                hash: ckb_types::H256::default(),
            }),
        };

        Ok(CellData {
            out_point: out_point.clone(),
            cell: CellWithStatus {
                cell: Some(cell_info),
                status: "live".to_string(),
            },
        })
    }

    /// Expand all DepGroups and batch-fetch their referenced cells.
    async fn expand_all_dep_groups(
        &self,
        tx: &Transaction,
        dep_cells: &[CellData],
    ) -> Result<Vec<CellData>> {
        let mut all_out_points: Vec<OutPoint> = Vec::new();

        for (i, cell_dep) in tx.cell_deps.iter().enumerate() {
            if cell_dep.dep_type != ckb_jsonrpc_types::DepType::DepGroup {
                continue;
            }

            let out_points = self.parse_dep_group_out_points(&dep_cells[i].cell, i)?;
            all_out_points.extend(out_points);
        }

        if all_out_points.is_empty() {
            return Ok(Vec::new());
        }

        debug!(
            "batch-fetching {} expanded dep group cells",
            all_out_points.len()
        );

        let batch_results = self
            .rpc
            .batch_get_live_cells(&all_out_points, true)
            .await
            .context("failed to batch-fetch expanded dep group cells")?;

        self.resolve_cells_with_fallback(&all_out_points, batch_results, "expanded dep")
            .await
    }

    /// Parse OutPoints from a DepGroup cell's data.
    ///
    /// DepGroup data is a molecule FixVec of OutPoints:
    /// - 4 bytes: item count (little-endian u32)
    /// - N * 36 bytes: OutPoints (32 bytes tx_hash + 4 bytes index LE)
    fn parse_dep_group_out_points(
        &self,
        dep_group_cell: &CellWithStatus,
        dep_index: usize,
    ) -> Result<Vec<OutPoint>> {
        let cell_info = dep_group_cell
            .cell
            .as_ref()
            .ok_or_else(|| anyhow!("dep group cell {} has no cell info", dep_index))?;

        let cell_data = cell_info
            .data
            .as_ref()
            .ok_or_else(|| anyhow!("dep group cell {} has no data", dep_index))?;

        let data_bytes = cell_data.content.as_bytes();

        if data_bytes.len() < 4 {
            return Err(anyhow!("dep group cell {} data too short", dep_index));
        }

        // First 4 bytes = item count (molecule FixVec header)
        let count = u32::from_le_bytes([
            data_bytes[0],
            data_bytes[1],
            data_bytes[2],
            data_bytes[3],
        ]) as usize;

        debug!("expanding dep group {} with {} out_points", dep_index, count);

        let expected_len = 4 + count * 36;
        if data_bytes.len() < expected_len {
            return Err(anyhow!(
                "dep group cell {} data length {} < expected {}",
                dep_index,
                data_bytes.len(),
                expected_len
            ));
        }

        let mut out_points = Vec::with_capacity(count);
        for i in 0..count {
            let offset = 4 + i * 36;

            let mut tx_hash_bytes = [0u8; 32];
            tx_hash_bytes.copy_from_slice(&data_bytes[offset..offset + 32]);

            let index_bytes = &data_bytes[offset + 32..offset + 36];
            let index = u32::from_le_bytes([
                index_bytes[0],
                index_bytes[1],
                index_bytes[2],
                index_bytes[3],
            ]);

            out_points.push(OutPoint {
                tx_hash: ckb_types::H256(tx_hash_bytes),
                index: ckb_jsonrpc_types::Uint32::from(index),
            });
        }

        Ok(out_points)
    }

    /// Convert an H256 block hash to a packed Byte32.
    fn h256_to_byte32(hash: &ckb_types::H256) -> Byte32 {
        let bytes: [u8; 32] = hash.clone().into();
        Byte32::new(bytes)
    }

    /// Convert JSON-RPC out_point to packed format.
    fn convert_out_point(&self, out_point: &OutPoint) -> Result<PackedOutPoint> {
        let tx_hash_bytes: [u8; 32] = out_point.tx_hash.clone().into();
        let index: u32 = out_point.index.into();

        Ok(PackedOutPoint::new_builder()
            .tx_hash(Byte32::new(tx_hash_bytes))
            .index(Uint32::from_slice(&index.to_le_bytes()).expect("valid u32"))
            .build())
    }

    /// Convert JSON-RPC cell to packed output and data.
    fn convert_cell(
        &self,
        cell: &ckb_jsonrpc_types::CellOutput,
    ) -> Result<(CellOutput, Bytes)> {
        let capacity: u64 = cell.capacity.into();

        let lock_script = self.convert_script(&cell.lock)?;

        let type_script = cell
            .type_
            .as_ref()
            .map(|s| self.convert_script(s))
            .transpose()?;

        let output = CellOutput::new_builder()
            .capacity(Uint64::from_slice(&capacity.to_le_bytes()).expect("valid u64"))
            .lock(lock_script)
            .type_(type_script.pack())
            .build();

        // Data will be fetched separately
        let data = Bytes::default();

        Ok((output, data))
    }

    /// Convert JSON-RPC script to packed format.
    fn convert_script(&self, script: &ckb_jsonrpc_types::Script) -> Result<Script> {
        let code_hash_bytes: [u8; 32] = script.code_hash.clone().into();

        let hash_type = self.convert_hash_type(&script.hash_type);

        let hash_type_byte: Byte = hash_type.into();
        Ok(Script::new_builder()
            .code_hash(Byte32::new(code_hash_bytes))
            .hash_type(hash_type_byte)
            .args(Bytes::copy_from_slice(script.args.as_bytes()).pack())
            .build())
    }

    /// Build core transaction from JSON-RPC data.
    fn build_core_transaction(&self, tx_data: &TransactionData) -> Result<CoreTransactionView> {
        let mut builder = TransactionBuilder::default();

        // Add inputs
        for input in &tx_data.tx.inputs {
            let out_point = self.convert_out_point(&input.previous_output)?;
            let since: u64 = input.since.into();
            let cell_input = CellInput::new_builder()
                .previous_output(out_point)
                .since(Uint64::from_slice(&since.to_le_bytes()).expect("valid u64"))
                .build();
            builder = builder.input(cell_input);
        }

        // Add outputs
        for output in &tx_data.tx.outputs {
            let (cell_output, _) = self.convert_cell(output)?;
            builder = builder.output(cell_output);
        }

        // Add outputs data
        for data in &tx_data.tx.outputs_data {
            builder = builder.output_data(Bytes::copy_from_slice(data.as_bytes()).pack());
        }

        // Add cell deps
        for dep in &tx_data.tx.cell_deps {
            let out_point = self.convert_out_point(&dep.out_point)?;
            let dep_type = match dep.dep_type {
                ckb_jsonrpc_types::DepType::Code => DepType::Code,
                ckb_jsonrpc_types::DepType::DepGroup => DepType::DepGroup,
            };
            let dep_type_byte: Byte = dep_type.into();
            let cell_dep = CellDep::new_builder()
                .out_point(out_point)
                .dep_type(dep_type_byte)
                .build();
            builder = builder.cell_dep(cell_dep);
        }

        // Add header deps
        for header_hash in &tx_data.tx.header_deps {
            let hash_bytes: [u8; 32] = header_hash.clone().into();
            builder = builder.header_dep(Byte32::new(hash_bytes));
        }

        // Add witnesses
        for witness in &tx_data.tx.witnesses {
            builder = builder.witness(Bytes::copy_from_slice(witness.as_bytes()).pack());
        }

        Ok(builder.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_group_type_constants() {
        assert_eq!(SCRIPT_GROUP_LOCK, 0);
        assert_eq!(SCRIPT_GROUP_TYPE, 1);
    }
}
