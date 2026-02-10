use ckb_vm::cost_model::constant_cycles;
use ckb_vm::elf::parse_elf;
use ckb_vm::machine::asm::{AsmCoreMachine, AsmDefaultMachineBuilder, AsmMachine};
use ckb_vm::machine::trace::TraceMachine;
use ckb_vm::machine::{CoreMachine, DefaultCoreMachine, DefaultMachine, DefaultMachineRunner, SupportMachine};
use ckb_vm::memory::{sparse::SparseMemory, wxorx::WXorXMemory};
use ckb_vm::snapshot2::{DataSource, Snapshot2, Snapshot2Context};
use ckb_vm::{Bytes, ISA_MOP, Memory};
use ckb_vm::{Error, ISA_B, ISA_IMC, RustDefaultMachineBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::u64;

pub enum MachineTy {
    Asm,
    Interpreter,
    InterpreterWithTrace,
}

impl MachineTy {
    pub fn build(self, data_source: TestSource, version: u32) -> Machine {
        match self {
            MachineTy::Asm => {
                let context = Arc::new(Mutex::new(Snapshot2Context::new(data_source)));
                let asm_core1 = <AsmCoreMachine as SupportMachine>::new(ISA_IMC | ISA_B | ISA_MOP, version, 0);
                let core1 =
                    AsmDefaultMachineBuilder::new(asm_core1).instruction_cycle_func(Box::new(constant_cycles)).build();
                Machine::Asm(AsmMachine::new(core1), context)
            }
            MachineTy::Interpreter => {
                let context = Arc::new(Mutex::new(Snapshot2Context::new(data_source)));
                let core_machine1 = DefaultCoreMachine::<u64, WXorXMemory<SparseMemory<u64>>>::new(
                    ISA_IMC | ISA_B | ISA_MOP,
                    version,
                    0,
                );
                Machine::Interpreter(
                    RustDefaultMachineBuilder::<DefaultCoreMachine<u64, WXorXMemory<SparseMemory<u64>>>>::new(
                        core_machine1,
                    )
                    .instruction_cycle_func(Box::new(constant_cycles))
                    .build(),
                    context,
                )
            }
            MachineTy::InterpreterWithTrace => {
                let context = Arc::new(Mutex::new(Snapshot2Context::new(data_source)));
                let core_machine1 = DefaultCoreMachine::<u64, WXorXMemory<SparseMemory<u64>>>::new(
                    ISA_IMC | ISA_B | ISA_MOP,
                    version,
                    0,
                );
                Machine::InterpreterWithTrace(
                    TraceMachine::new(
                        RustDefaultMachineBuilder::<DefaultCoreMachine<u64, WXorXMemory<SparseMemory<u64>>>>::new(
                            core_machine1,
                        )
                        .instruction_cycle_func(Box::new(constant_cycles))
                        .build(),
                    ),
                    context,
                )
            }
        }
    }
}

pub enum Machine {
    Asm(AsmMachine, Arc<Mutex<Snapshot2Context<u64, TestSource>>>),
    Interpreter(
        DefaultMachine<DefaultCoreMachine<u64, WXorXMemory<SparseMemory<u64>>>>,
        Arc<Mutex<Snapshot2Context<u64, TestSource>>>,
    ),
    InterpreterWithTrace(
        TraceMachine<DefaultCoreMachine<u64, WXorXMemory<SparseMemory<u64>>>>,
        Arc<Mutex<Snapshot2Context<u64, TestSource>>>,
    ),
}

impl Machine {
    pub fn load_program(&mut self, args: impl ExactSizeIterator<Item = Result<Bytes, Error>>) -> Result<u64, Error> {
        use Machine::*;
        match self {
            Asm(inner, context) => {
                let (program, _) = context.lock().unwrap().load_data(&PROGRAM_ID, 0, 0).unwrap();
                let metadata = parse_elf::<u64>(&program, inner.machine.version())?;
                let bytes = inner.load_program_with_metadata(&program, &metadata, args)?;
                context.lock().unwrap().mark_program(inner.machine.inner_mut(), &metadata, &PROGRAM_ID, 0)?;
                Ok(bytes)
            }
            Interpreter(inner, context) => {
                let (program, _) = context.lock().unwrap().load_data(&PROGRAM_ID, 0, 0).unwrap();
                let metadata = parse_elf::<u64>(&program, inner.version())?;
                let bytes = inner.load_program_with_metadata(&program, &metadata, args)?;
                context.lock().unwrap().mark_program(inner.inner_mut(), &metadata, &PROGRAM_ID, 0)?;
                Ok(bytes)
            }
            InterpreterWithTrace(inner, context) => {
                let (program, _) = context.lock().unwrap().load_data(&PROGRAM_ID, 0, 0).unwrap();
                let metadata = parse_elf::<u64>(&program, inner.machine.version())?;
                let bytes = inner.load_program_with_metadata(&program, &metadata, args)?;
                context.lock().unwrap().mark_program(inner.machine.inner_mut(), &metadata, &PROGRAM_ID, 0)?;
                Ok(bytes)
            }
        }
    }

    pub fn run(&mut self) -> Result<i8, Error> {
        use Machine::*;
        match self {
            Asm(inner, _) => inner.run(),
            Interpreter(inner, _) => inner.run(),
            InterpreterWithTrace(inner, _) => inner.run(),
        }
    }

    pub fn set_max_cycles(&mut self, cycles: u64) {
        use Machine::*;
        match self {
            Asm(inner, _) => inner.machine.set_max_cycles(cycles),
            Interpreter(inner, _) => inner.set_max_cycles(cycles),
            InterpreterWithTrace(inner, _) => inner.machine.set_max_cycles(cycles),
        }
    }

    pub fn cycles(&self) -> u64 {
        use Machine::*;
        match self {
            Asm(inner, _) => inner.machine.cycles(),
            Interpreter(inner, _) => inner.cycles(),
            InterpreterWithTrace(inner, _) => inner.machine.cycles(),
        }
    }

    pub fn full_memory(&mut self) -> Result<Bytes, Error> {
        use Machine::*;
        use ckb_vm::DEFAULT_MEMORY_SIZE;
        match self {
            Asm(inner, _) => inner.machine.memory_mut().load_bytes(0, DEFAULT_MEMORY_SIZE as u64),
            Interpreter(inner, _) => inner.memory_mut().load_bytes(0, DEFAULT_MEMORY_SIZE as u64),
            InterpreterWithTrace(inner, _) => inner.machine.memory_mut().load_bytes(0, DEFAULT_MEMORY_SIZE as u64),
        }
    }

    pub fn full_registers(&self) -> [u64; 33] {
        use Machine::*;
        let mut regs = [0u64; 33];
        match self {
            Asm(inner, _) => {
                regs[0..32].copy_from_slice(inner.machine.registers());
                regs[32] = *inner.machine.pc();
            }
            Interpreter(inner, _) => {
                regs[0..32].copy_from_slice(inner.registers());
                regs[32] = *inner.pc();
            }
            InterpreterWithTrace(inner, _) => {
                regs[0..32].copy_from_slice(inner.machine.registers());
                regs[32] = *inner.machine.pc();
            }
        };
        regs
    }

    pub fn snapshot(&mut self) -> Result<Snapshot2<u64>, Error> {
        use Machine::*;
        match self {
            Asm(inner, context) => {
                let context = context.lock().unwrap();
                Ok(context.make_snapshot(inner.machine.inner_mut())?)
            }
            Interpreter(inner, context) => {
                let context = context.lock().unwrap();
                Ok(context.make_snapshot(inner.inner_mut())?)
            }
            InterpreterWithTrace(inner, context) => {
                let context = context.lock().unwrap();
                Ok(context.make_snapshot(inner.machine.inner_mut())?)
            }
        }
    }

    pub fn resume(&mut self, snap: Snapshot2<u64>) -> Result<(), Error> {
        use Machine::*;
        match self {
            Asm(inner, context) => {
                context.lock().unwrap().resume(inner.machine.inner_mut(), &snap)?;
            }
            Interpreter(inner, context) => {
                context.lock().unwrap().resume(inner.inner_mut(), &snap)?;
            }
            InterpreterWithTrace(inner, context) => {
                context.lock().unwrap().resume(inner.machine.inner_mut(), &snap)?;
            }
        };
        Ok(())
    }
}

pub fn load_program(name: &str) -> TestSource {
    let mut file = File::open(name).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let program = buffer.into();

    let mut data = vec![0; 16 * 4096];
    for i in 0..data.len() {
        data[i] = i as u8;
    }

    let mut m = HashMap::default();
    m.insert(DATA_ID, data.into());
    m.insert(PROGRAM_ID, program);

    TestSource(m)
}

pub const PROGRAM_ID: u64 = 0x1234;
pub const DATA_ID: u64 = 0x2000;

#[derive(Clone)]
pub struct TestSource(HashMap<u64, Bytes>);

impl DataSource<u64> for TestSource {
    fn load_data(&self, id: &u64, offset: u64, length: u64) -> Option<(Bytes, u64)> {
        match self.0.get(id) {
            Some(data) => {
                let end = if length > 0 { offset + length } else { data.len() as u64 };
                let full_length = end - offset;
                Some((data.slice(offset as usize..end as usize), full_length))
            }
            None => None,
        }
    }
}
