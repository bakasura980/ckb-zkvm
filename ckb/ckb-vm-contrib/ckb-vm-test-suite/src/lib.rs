use ckb_vm::snapshot2::Snapshot2;
use ckb_vm::{
    Bytes, DefaultMachineRunner, Error, ISA_B, ISA_IMC, ISA_MOP, SparseMemory, SupportMachine,
    machine::{
        VERSION2,
        asm::{AsmCoreMachine, AsmDefaultMachineBuilder, AsmMachine},
    },
    run,
};
use std::cell::LazyCell;
use std::path::Path;
use std::process::Command;

pub mod snapshot2_help;

pub const BINARY_PATH_ED25519: &str = "programs/build/release/ed25519_ckbvm";
pub const BINARY_PATH_K256_ECDSA: &str = "programs/build/release/k256_ecdsa_ckbvm";
pub const BINARY_PATH_K256_SCHNORR: &str = "programs/build/release/k256_schnorr_ckbvm";
pub const BINARY_PATH_P256: &str = "programs/build/release/p256_ckbvm";
pub const BINARY_PATH_RSA: &str = "programs/build/release/rsa_ckbvm";
pub const BINARY_PATH_SECP256K1_ECDSA: &str = "programs/build/release/secp256k1_ecdsa_ckbvm";
pub const BINARY_PATH_SECP256K1_SCHNORR: &str = "programs/build/release/secp256k1_schnorr_ckbvm";
pub const BINARY_PATH_SPHINCSPLUS_REF: &str = "programs/build/release/sphincsplus_ref_ckbvm";
pub const NATIVE_PATH_ED25519: &str = "programs/build/release/ed25519_native";
pub const NATIVE_PATH_K256_ECDSA: &str = "programs/build/release/k256_ecdsa_native";
pub const NATIVE_PATH_K256_SCHNORR: &str = "programs/build/release/k256_schnorr_native";
pub const NATIVE_PATH_P256: &str = "programs/build/release/p256_native";
pub const NATIVE_PATH_RSA: &str = "programs/build/release/rsa_native";
pub const NATIVE_PATH_SECP256K1_ECDSA: &str = "programs/build/release/secp256k1_ecdsa_native";
pub const NATIVE_PATH_SECP256K1_SCHNORR: &str = "programs/build/release/secp256k1_schnorr_native";
pub const NATIVE_PATH_SPHINCSPLUS_REF: &str = "programs/build/release/sphincsplus_ref_native";

pub const NTIMES: LazyCell<String> = LazyCell::new(|| std::env::var("NTIMES").unwrap_or(String::from("1")));

pub fn run_asm(program: &Bytes) {
    let asm_core = AsmCoreMachine::new(ISA_IMC | ISA_B, VERSION2, u64::MAX);
    let core = AsmDefaultMachineBuilder::new(asm_core).build();
    let mut machine = AsmMachine::new(core);
    let args = vec![Bytes::copy_from_slice(&NTIMES.clone().as_bytes())];
    machine.load_program(&program, args.into_iter().map(Ok)).unwrap();
    let exit = machine.run().unwrap();
    assert_eq!(exit, 0);
}

pub fn run_interpret(program: &Bytes) {
    let args = vec![Bytes::copy_from_slice(&NTIMES.clone().as_bytes())];
    let exit = run::<u64, SparseMemory<u64>>(&program, &args).unwrap();
    assert_eq!(exit, 0);
}

pub fn run_mop(program: &Bytes) {
    let asm_core = AsmCoreMachine::new(ISA_IMC | ISA_B | ISA_MOP, VERSION2, u64::MAX);
    let core = AsmDefaultMachineBuilder::new(asm_core).build();
    let mut machine = AsmMachine::new(core);
    let args = vec![Bytes::copy_from_slice(&NTIMES.clone().as_bytes())];
    machine.load_program(&program, args.into_iter().map(Ok)).unwrap();
    let exit = machine.run().unwrap();
    assert_eq!(exit, 0);
}

pub fn run_native<P: AsRef<Path>>(path: P) {
    assert!(Command::new(path.as_ref()).arg(NTIMES.as_str()).status().unwrap().success());
}

pub fn run_snapshot2<P: AsRef<Path>>(path: P) {
    let data_source = snapshot2_help::load_program(path.as_ref().to_str().unwrap());
    let mut snapshot2: Option<Snapshot2<u64>>;
    let per_cycles = 10000 + rand::random::<u64>() % 10000;
    let mut cycles = per_cycles;

    let mut machine = snapshot2_help::MachineTy::Asm.build(data_source.clone(), VERSION2);
    machine.set_max_cycles(cycles);
    let args = vec![Bytes::copy_from_slice(&NTIMES.clone().as_bytes())];
    machine.load_program(args.into_iter().map(Ok)).unwrap();
    let result = machine.run();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::CyclesExceeded);
    snapshot2 = Some(machine.snapshot().unwrap());
    loop {
        let mut machine = match rand::random::<u64>() % 3 {
            0 => snapshot2_help::MachineTy::Asm.build(data_source.clone(), VERSION2),
            1 => snapshot2_help::MachineTy::Interpreter.build(data_source.clone(), VERSION2),
            2 => snapshot2_help::MachineTy::InterpreterWithTrace.build(data_source.clone(), VERSION2),
            _ => unreachable!(),
        };
        machine.resume(snapshot2.take().unwrap()).unwrap();
        cycles += per_cycles;
        machine.set_max_cycles(cycles);
        let result = machine.run();
        match result {
            Ok(exit) => {
                assert_eq!(exit, 0);
                break;
            }
            Err(err) => {
                assert_eq!(err, Error::CyclesExceeded);
                snapshot2 = Some(machine.snapshot().unwrap());
            }
        }
    }
}
