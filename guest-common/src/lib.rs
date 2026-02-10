//! Shared CKB-VM replay infrastructure for SP1 guest programs.
//!
//! This crate provides the common replay logic used by all guest programs
//! (`program`, `program-64`, `guest-single`). The core functionality is
//! replaying CKB script execution traces through CKB-VM using recorded
//! syscall responses.

use ckb_vm::{
    Bytes, DefaultCoreMachine, DefaultMachineBuilder, Error as VMError, Register, SupportMachine,
    ISA_A, ISA_B, ISA_IMC, ISA_MOP,
    machine::VERSION2,
    memory::sparse::SparseMemory,
    registers::A7,
    syscalls::Syscalls,
};
use ckb_vm_fuzzing_utils::SynchronousSyscalls;
use protobuf_ckb_syscalls::ProtobufVmRunnerImpls;

use ckb_zkvm_lib::{ExecutionTrace, ELF_MAGIC};

/// RISC-V ISA flags for CKB VM (base + all extensions).
pub const ISA_FLAGS: u8 = ISA_IMC | ISA_A | ISA_B | ISA_MOP;

/// Maximum cycles for VM execution. Set to u64::MAX because cycle counting
/// is disabled (instruction_cycle_func returns 0). Execution terminates on
/// program exit, not cycle exhaustion.
pub const MAX_CYCLES: u64 = u64::MAX;

/// Type alias for the core machine type used in replay.
pub type CoreMachineType = DefaultCoreMachine<u64, SparseMemory<u64>>;

/// CKB exec syscall number (ecall code in register A7).
pub const CKB_EXEC_SYSCALL: u64 = 2043;

/// Wrapper around SynchronousSyscalls that intercepts the exec syscall.
///
/// In real CKB VM, exec (2043) replaces the running program with a new binary.
/// During replay we only have the first generation's ELF, so we can't load the
/// exec'd program. Instead, we halt the VM with exit code 0 — the on-chain
/// CKB consensus already verified the full exec chain.
///
/// This is consistent with how spawn is handled: the parent trace is replayed
/// but child VMs are not re-executed in the guest.
pub struct ExecHaltingSyscalls<S, M> {
    inner: SynchronousSyscalls<S, M>,
}

impl<S, M> Syscalls<M> for ExecHaltingSyscalls<S, M>
where
    S: Send,
    M: SupportMachine + Send,
    SynchronousSyscalls<S, M>: Syscalls<M>,
{
    fn initialize(&mut self, machine: &mut M) -> Result<(), VMError> {
        self.inner.initialize(machine)
    }

    fn ecall(&mut self, machine: &mut M) -> Result<bool, VMError> {
        if machine.registers()[A7].to_u64() == CKB_EXEC_SYSCALL {
            machine.set_running(false);
            return Ok(true);
        }
        self.inner.ecall(machine)
    }
}

/// Replay the execution trace through CKB-VM.
///
/// Panics if the script exits with a non-zero code, since confirmed
/// on-chain transactions have already passed all script validations.
///
/// Takes ownership of the trace to move the ELF binary into the VM
/// without copying. Uses ProtobufVmRunnerImpls to return recorded
/// syscall responses instead of executing them.
pub fn replay_trace(trace: ExecutionTrace) {
    // Validate trace has required data
    assert!(!trace.elf.is_empty(), "trace missing ELF binary");
    assert!(
        trace.has_syscall_trace(),
        "trace missing syscall data for replay"
    );

    // Validate ELF magic bytes
    assert!(
        trace.elf.len() >= 4 && trace.elf[0..4] == ELF_MAGIC,
        "invalid ELF magic bytes"
    );

    // Create replay implementation from syscall trace bytes.
    // ProtobufVmRunnerImpls<M> decodes the protobuf Syscalls message
    // and creates a FIFO queue of recorded responses.
    let replayer: ProtobufVmRunnerImpls<CoreMachineType> =
        ProtobufVmRunnerImpls::new_with_bytes(trace.syscall_trace_bytes())
            .expect("failed to create syscall replayer from trace");

    // Build argv from replayer args before moving replayer into the syscall handler.
    let argv: Vec<Result<Bytes, ckb_vm::Error>> = replayer
        .args()
        .iter()
        .map(|a| Ok(Bytes::copy_from_slice(a)))
        .collect();

    let core_machine = CoreMachineType::new(
        ISA_FLAGS,
        VERSION2,
        MAX_CYCLES,
    );

    let syscall_handler = ExecHaltingSyscalls {
        inner: SynchronousSyscalls::new(replayer),
    };

    // Cycle counting disabled — only the exit code matters for proof correctness.
    let mut machine = DefaultMachineBuilder::new(core_machine)
        .instruction_cycle_func(Box::new(|_| 0))
        .syscall(Box::new(syscall_handler))
        .build();

    let program_bytes = Bytes::from(trace.elf);

    machine
        .load_program(&program_bytes, argv.into_iter())
        .expect("failed to load program into VM");

    // Run the VM - the syscall handler will return recorded responses
    match machine.run() {
        Ok(0) => {}
        Ok(code) => panic!("script exited with non-zero code: {}", code),
        Err(e) => panic!("VM execution error: {:?}", e),
    }
}
