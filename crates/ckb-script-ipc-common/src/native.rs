extern crate std;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;

use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::registers::{A0, A1, A2, A7};
use ckb_vm::{Bytes, CoreMachine, Memory, Register, SupportMachine, Syscalls};

use crate::io::{Error, ErrorKind, Read, Write};

pub const SPAWN: i32 = 2601;
pub const WAIT: i32 = 2602;
pub const PROCESS_ID: i32 = 2603;
pub const PIPE: i32 = 2604;
pub const WRITE: i32 = 2605;
pub const READ: i32 = 2606;
pub const INHERITED_FD: i32 = 2607;
pub const CLOSE: i32 = 2608;
pub const DEBUG_PRINT_SYSCALL_NUMBER: i32 = 2177;

pub const SPAWN_YIELD_CYCLES_BASE: u64 = 800;

pub const FIRST_FD_SLOT: u64 = 2;

pub struct DebugSyscall {}

impl<Mac: SupportMachine> Syscalls<Mac> for DebugSyscall {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::error::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::error::Error> {
        let code = &machine.registers()[A7];
        let code = code.to_i32();

        // in native implementation, we don't support these syscalls
        if code == SPAWN || code == WAIT || code == PROCESS_ID || code == PIPE {
            return Err(ckb_vm::error::Error::Unexpected(
                "unsupported syscalls: spawn, wait, process_id and pipe".into(),
            ));
        }
        if code != DEBUG_PRINT_SYSCALL_NUMBER {
            return Ok(false);
        }

        let mut addr = machine.registers()[A0].to_u64();
        let mut buffer = Vec::new();

        loop {
            let byte = machine
                .memory_mut()
                .load8(&Mac::REG::from_u64(addr))?
                .to_u8();
            if byte == 0 {
                break;
            }
            buffer.push(byte);
            addr += 1;
        }

        let s = String::from_utf8(buffer).unwrap();
        std::println!("{:?}", s);
        machine.set_register(A0, Mac::REG::from_u8(0));
        Ok(true)
    }
}
pub struct ReadSyscall {
    pipe: Pipe,
}

impl ReadSyscall {
    pub fn new(pipe: Pipe) -> Self {
        Self { pipe }
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for ReadSyscall {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::error::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::error::Error> {
        let code = &machine.registers()[A7];
        if code.to_i32() != READ {
            return Ok(false);
        }
        let fd = machine.registers()[A0].to_u64();
        if fd != FIRST_FD_SLOT {
            return Err(ckb_vm::error::Error::Unexpected(
                "can only read on pipe 2".into(),
            ));
        }
        let buffer_addr = machine.registers()[A1].clone();
        let length_addr = machine.registers()[A2].clone();
        let length = machine.memory_mut().load64(&length_addr)?.to_u64() as usize;
        let mut buf = vec![0; length];
        let real_len = self
            .pipe
            .read(&mut buf)
            .map_err(|_| ckb_vm::error::Error::Unexpected("READ error".into()))?;
        machine
            .memory_mut()
            .store_bytes(buffer_addr.to_u64(), &buf[..real_len])?;
        machine
            .memory_mut()
            .store64(&length_addr, &Mac::REG::from_u64(real_len as u64))?;
        #[cfg(feature = "enable-logging")]
        log::info!("Syscall Read: read {} bytes", real_len);
        machine.add_cycles_no_checking(SPAWN_YIELD_CYCLES_BASE)?;
        machine.set_register(A0, Mac::REG::from_u8(0));
        Ok(true)
    }
}

pub struct WriteSyscall {
    pipe: Pipe,
}

impl WriteSyscall {
    pub fn new(pipe: Pipe) -> Self {
        Self { pipe }
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for WriteSyscall {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::error::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::error::Error> {
        let code = &machine.registers()[A7];
        if code.to_i32() != WRITE {
            return Ok(false);
        }
        let fd = machine.registers()[A0].to_u64();
        if fd != (FIRST_FD_SLOT + 1) {
            return Err(ckb_vm::error::Error::Unexpected(
                "can only write on pipe 3".into(),
            ));
        }
        let buffer_addr = machine.registers()[A1].clone();
        let length_addr = machine.registers()[A2].clone();
        let length = machine.memory_mut().load64(&length_addr)?.to_u64();
        // Skip zero-length writes to prevent false EOF signals
        // The ckb-script scheduler allows zero-length data transfers, which could be
        // misinterpreted as EOF by higher-level APIs
        if length == 0 {
            machine.set_register(A0, Mac::REG::from_u8(0));
            return Ok(true);
        }
        let bytes = machine
            .memory_mut()
            .load_bytes(buffer_addr.to_u64(), length)?;
        // the pipe write can't write partial data so we don't need to check result length.
        self.pipe
            .write(&bytes)
            .map_err(|_| ckb_vm::error::Error::Unexpected("WRITE error".into()))?;

        machine
            .memory_mut()
            .store64(&length_addr, &Mac::REG::from_u64(bytes.len() as u64))?;
        #[cfg(feature = "enable-logging")]
        log::info!("Syscall Write: write {} bytes", bytes.len());
        machine.add_cycles_no_checking(SPAWN_YIELD_CYCLES_BASE)?;
        machine.set_register(A0, Mac::REG::from_u8(0));
        Ok(true)
    }
}

pub struct InheritedFdSyscall {}

impl<Mac: SupportMachine> Syscalls<Mac> for InheritedFdSyscall {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::error::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::error::Error> {
        let code = &machine.registers()[A7];
        if code.to_i32() != INHERITED_FD {
            return Ok(false);
        }
        let buffer_addr = machine.registers()[A0].clone();
        let length_addr = machine.registers()[A1].clone();

        let length = machine.memory_mut().load64(&length_addr)?;
        if length.to_u64() < 2 {
            return Err(ckb_vm::error::Error::Unexpected(
                "length of inherited fd is less than 2".into(),
            ));
        }
        let mut inherited_fd = [0u8; 16];
        inherited_fd[0..8].copy_from_slice(&FIRST_FD_SLOT.to_le_bytes());
        inherited_fd[8..16].copy_from_slice(&(FIRST_FD_SLOT + 1).to_le_bytes());
        machine
            .memory_mut()
            .store_bytes(buffer_addr.to_u64(), &inherited_fd[..])?;
        machine
            .memory_mut()
            .store64(&length_addr, &Mac::REG::from_u64(2))?;

        machine.set_register(A0, Mac::REG::from_u8(0));
        machine.add_cycles_no_checking(SPAWN_YIELD_CYCLES_BASE)?;
        Ok(true)
    }
}

pub struct CloseSyscall {}

impl<Mac: SupportMachine> Syscalls<Mac> for CloseSyscall {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::error::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::error::Error> {
        let code = &machine.registers()[A7];
        if code.to_i32() != CLOSE {
            return Ok(false);
        }
        machine.set_register(A0, Mac::REG::from_u8(0));
        Ok(true)
    }
}

pub struct Pipe {
    tx: Option<SyncSender<Vec<u8>>>,
    rx: Option<Mutex<Receiver<Vec<u8>>>>,
    buf: Vec<u8>,
}

impl Pipe {
    pub fn new_pair() -> (Self, Self) {
        let (tx, rx) = sync_channel(0);
        (
            Self {
                tx: None,
                rx: Some(Mutex::new(rx)),
                buf: vec![],
            },
            Self {
                tx: Some(tx),
                rx: None,
                buf: vec![],
            },
        )
    }
    pub fn close(&mut self) {
        if self.tx.is_some() {
            drop(self.tx.take());
        }
        if self.rx.is_some() {
            drop(self.rx.take());
        }
    }
}

impl Read for Pipe {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if self.buf.is_empty() {
            if let Some(rx) = &self.rx {
                match rx.lock().unwrap().recv() {
                    Ok(data) => self.buf = data,
                    Err(_) => {
                        return Err(Error::new(ErrorKind::UnexpectedEof, "channel is closed"))
                    }
                }
            } else {
                panic!("rx is none");
            }
        }

        let len = self.buf.len().min(buf.len());
        buf[..len].copy_from_slice(&self.buf[..len]);
        self.buf = self.buf.split_off(len);
        #[cfg(feature = "enable-logging")]
        log::info!("Pipe Read: read {} bytes", len);
        Ok(len)
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        // short-circuit zero-length writes
        if buf.is_empty() {
            return Ok(0);
        }
        self.tx.as_mut().unwrap().send(buf.to_vec()).unwrap();
        #[cfg(feature = "enable-logging")]
        log::info!("Pipe Write: write {} bytes", buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

fn main_int(
    code: Bytes,
    args: Vec<Bytes>,
    read_pipe: Pipe,
    write_pipe: Pipe,
) -> Result<(), Box<dyn std::error::Error>> {
    let core_machine = ckb_vm::DefaultCoreMachine::<u64, ckb_vm::SparseMemory<u64>>::new(
        ckb_vm::ISA_IMC | ckb_vm::ISA_B | ckb_vm::ISA_MOP,
        ckb_vm::machine::VERSION2,
        u64::MAX,
    );
    let machine_builder = ckb_vm::DefaultMachineBuilder::new(core_machine)
        .instruction_cycle_func(Box::new(estimate_cycles));
    let mut machine = machine_builder
        .syscall(Box::new(DebugSyscall {}))
        .syscall(Box::new(ReadSyscall::new(read_pipe)))
        .syscall(Box::new(WriteSyscall::new(write_pipe)))
        .syscall(Box::new(InheritedFdSyscall {}))
        .syscall(Box::new(CloseSyscall {}))
        .build();
    machine.load_program(&code, &args)?;
    let exit = machine.run();
    let cycles = machine.cycles();
    std::println!(
        "int exit={:?} cycles={:?} r[a1]={:?}",
        exit,
        cycles,
        machine.registers()[ckb_vm::registers::A1]
    );
    Ok(())
}

pub fn spawn_server(
    script_binary: &[u8],
    args: &[&str],
) -> Result<(Pipe, Pipe), Box<dyn core::error::Error>> {
    // channel: ckb-vm -> native
    let (read_pipe1, write_pipe1) = Pipe::new_pair();
    // channel: native -> ckb-vm
    let (read_pipe2, write_pipe2) = Pipe::new_pair();

    let code = Bytes::copy_from_slice(script_binary);
    let args = args
        .iter()
        .map(|s| Bytes::copy_from_slice(s.as_bytes()))
        .collect();
    // TODO: shutdown service gracefully.
    std::thread::spawn(move || {
        let _ = main_int(code, args, read_pipe2, write_pipe1);
    });
    Ok((read_pipe1, write_pipe2))
}
