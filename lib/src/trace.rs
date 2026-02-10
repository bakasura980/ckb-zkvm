//! Execution trace types for CKB transaction replay.
//!
//! These types are generated from proto/trace.proto and wrapped
//! with additional convenience methods.

use prost::Message;

use crate::{Error, Result, H256};

// Include generated protobuf types
include!(concat!(env!("OUT_DIR"), "/ckb_zkvm.rs"));

impl ExecutionTrace {
    /// Create a new execution trace.
    pub fn new(tx_hash: H256, elf: Vec<u8>, args: Vec<Vec<u8>>) -> Self {
        Self {
            tx_hash: tx_hash.to_vec(),
            elf,
            args,
            syscall_trace: Vec::new(),
            vm_creations: Vec::new(),
            script_hash: Vec::new(),
            script_group_type: 0,
        }
    }

    /// Get the transaction hash as H256.
    pub fn tx_hash_h256(&self) -> Result<H256> {
        if self.tx_hash.len() != 32 {
            return Err(Error::InvalidTrace(format!(
                "tx_hash has invalid length: {}",
                self.tx_hash.len()
            )));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.tx_hash);
        Ok(hash)
    }

    /// Set the syscall trace bytes (serialized ckb-vm-contrib Syscalls).
    pub fn set_syscall_trace(&mut self, trace_bytes: Vec<u8>) {
        self.syscall_trace = trace_bytes;
    }

    /// Get the syscall trace bytes for replay.
    pub fn syscall_trace_bytes(&self) -> &[u8] {
        &self.syscall_trace
    }

    /// Add a VM creation event to the trace.
    pub fn add_vm_creation(&mut self, creation: VmCreation) {
        self.vm_creations.push(creation);
    }

    /// Get the script hash as H256.
    pub fn script_hash_h256(&self) -> Result<H256> {
        if self.script_hash.len() != 32 {
            return Err(Error::InvalidTrace(format!(
                "script_hash has invalid length: {}",
                self.script_hash.len()
            )));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.script_hash);
        Ok(hash)
    }

    /// Set the script hash being executed.
    pub fn set_script_hash(&mut self, hash: H256) {
        self.script_hash = hash.to_vec();
    }

    /// Set the script group type (0 = Lock, 1 = Type).
    pub fn set_script_group_type(&mut self, group_type: u32) {
        self.script_group_type = group_type;
    }

    /// Serialize the trace to protobuf bytes.
    pub fn encode_to_vec(&self) -> Vec<u8> {
        Message::encode_to_vec(self)
    }

    /// Deserialize a trace from protobuf bytes.
    pub fn decode_from_slice(bytes: &[u8]) -> Result<Self> {
        Self::decode(bytes).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Get the number of VM creations in the trace.
    pub fn vm_creation_count(&self) -> usize {
        self.vm_creations.len()
    }

    /// Check if the trace has syscall data.
    pub fn has_syscall_trace(&self) -> bool {
        !self.syscall_trace.is_empty()
    }

    /// Get the size of the syscall trace in bytes.
    pub fn syscall_trace_len(&self) -> usize {
        self.syscall_trace.len()
    }
}

impl VmCreation {
    /// Create a new VM creation event.
    pub fn new(
        vm_id: u64,
        generation_id: u64,
        parent_vm_id: Option<u64>,
        binary_locator: Option<BinaryLocator>,
    ) -> Self {
        Self {
            vm_id,
            generation_id,
            parent_vm_id,
            binary_locator,
        }
    }
}

impl BinaryLocator {
    /// Create a binary locator for inline data.
    pub fn inline(data: Vec<u8>) -> Self {
        Self {
            locator: Some(binary_locator::Locator::InlineData(data)),
        }
    }

    /// Create a binary locator for cell data.
    pub fn from_cell(tx_hash: H256, index: u32, offset: u64, length: u64) -> Self {
        Self {
            locator: Some(binary_locator::Locator::CellData(CellDataLocator {
                tx_hash: tx_hash.to_vec(),
                index,
                offset,
                length,
            })),
        }
    }

    /// Create a binary locator for dep group.
    pub fn from_dep_group(tx_hash: H256, index: u32, dep_index: u32) -> Self {
        Self {
            locator: Some(binary_locator::Locator::DepGroup(DepGroupLocator {
                tx_hash: tx_hash.to_vec(),
                index,
                dep_index,
            })),
        }
    }
}

impl TransactionTraces {
    /// Create a new transaction traces container.
    pub fn new(tx_hash: H256) -> Self {
        Self {
            tx_hash: tx_hash.to_vec(),
            script_traces: Vec::new(),
        }
    }

    /// Get the transaction hash as H256.
    pub fn tx_hash_h256(&self) -> Result<H256> {
        if self.tx_hash.len() != 32 {
            return Err(Error::InvalidTrace(format!(
                "tx_hash has invalid length: {}",
                self.tx_hash.len()
            )));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.tx_hash);
        Ok(hash)
    }

    /// Add a script trace to this transaction.
    pub fn add_script_trace(&mut self, trace: ExecutionTrace) {
        self.script_traces.push(trace);
    }

    /// Get the number of script traces.
    pub fn script_count(&self) -> usize {
        self.script_traces.len()
    }

    /// Get an iterator over script traces.
    pub fn iter(&self) -> impl Iterator<Item = &ExecutionTrace> {
        self.script_traces.iter()
    }

    /// Serialize to protobuf bytes.
    pub fn encode_to_vec(&self) -> Vec<u8> {
        Message::encode_to_vec(self)
    }

    /// Deserialize from protobuf bytes.
    pub fn decode_from_slice(bytes: &[u8]) -> Result<Self> {
        Self::decode(bytes).map_err(|e| Error::Serialization(e.to_string()))
    }
}

impl ScriptResult {
    /// Create a new script result.
    pub fn new(script_hash: H256, script_group_type: u32, exit_code: i64, cycles: u64) -> Self {
        Self {
            script_hash: script_hash.to_vec(),
            script_group_type,
            exit_code,
            cycles,
        }
    }

    /// Get the script hash as H256.
    pub fn script_hash_h256(&self) -> Result<H256> {
        if self.script_hash.len() != 32 {
            return Err(Error::InvalidTrace(format!(
                "script_hash has invalid length: {}",
                self.script_hash.len()
            )));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.script_hash);
        Ok(hash)
    }

    /// Check if this script executed successfully.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_trace_roundtrip() {
        let tx_hash = [1u8; 32];
        let elf = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic
        let args = vec![vec![1, 2, 3]];

        let mut trace = ExecutionTrace::new(tx_hash, elf.clone(), args.clone());
        trace.set_syscall_trace(vec![10, 20, 30]); // Placeholder syscall trace

        let encoded = trace.encode_to_vec();
        let decoded = ExecutionTrace::decode_from_slice(&encoded).unwrap();

        assert_eq!(decoded.tx_hash_h256().unwrap(), tx_hash);
        assert_eq!(decoded.elf, elf);
        assert_eq!(decoded.args, args);
        assert_eq!(decoded.syscall_trace_bytes(), &[10, 20, 30]);
    }

    #[test]
    fn test_binary_locator_inline() {
        let data = vec![1, 2, 3, 4];
        let locator = BinaryLocator::inline(data.clone());

        match locator.locator {
            Some(binary_locator::Locator::InlineData(d)) => assert_eq!(d, data),
            _ => panic!("expected inline data"),
        }
    }

    #[test]
    fn test_vm_creation() {
        let creation = VmCreation::new(1, 0, Some(0), None);
        assert_eq!(creation.vm_id, 1);
        assert_eq!(creation.generation_id, 0);
        assert_eq!(creation.parent_vm_id, Some(0));
    }
}
