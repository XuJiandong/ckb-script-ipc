extern crate std;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;

use ckb_vm::cost_model::estimate_cycles;
use ckb_vm::registers::{A0, A1, A2, A7};
use ckb_vm::{Bytes, Memory, Register, SupportMachine, Syscalls};

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
            return Err(ckb_vm::error::Error::IO {
                kind: std::io::ErrorKind::Other,
                data: "unsupported syscalls: spawn, wait, process_id and pipe".into(),
            });
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
            return Err(ckb_vm::error::Error::IO {
                kind: std::io::ErrorKind::Other,
                data: "can only read on pipe 2".into(),
            });
        }
        let buffer_addr = machine.registers()[A1].clone();
        let length_addr = machine.registers()[A2].clone();
        let length = machine.memory_mut().load64(&length_addr)?.to_u64() as usize;
        let mut buf = vec![0; length];
        let real_len = self
            .pipe
            .read(&mut buf)
            .map_err(|_| ckb_vm::error::Error::IO {
                kind: std::io::ErrorKind::Other,
                data: "READ error".into(),
            })?;
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
            return Err(ckb_vm::error::Error::IO {
                kind: std::io::ErrorKind::Other,
                data: "can only write on pipe 3".into(),
            });
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
            .map_err(|_| ckb_vm::error::Error::IO {
                kind: std::io::ErrorKind::Other,
                data: "WRITE error".into(),
            })?;

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
            return Err(ckb_vm::error::Error::IO {
                kind: std::io::ErrorKind::Other,
                data: "length of inherited fd is less than 2".into(),
            });
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

/// A bidirectional communication channel for transferring data between native code and CKB-VM.
///
/// `Pipe` implements a buffered channel that can be used for either reading or writing, but not both
/// simultaneously. It uses a synchronous channel internally to ensure proper flow control.
///
/// # Structure
/// * `tx` - Optional sender end of the channel
/// * `rx` - Optional receiver end of the channel, wrapped in a mutex for thread safety
/// * `buf` - Internal buffer for storing partially read data
///
/// # Examples
/// ```ignore
/// // Create a pipe pair for bidirectional communication
/// let (pipe1, pipe2) = Pipe::new_pair();
///
/// // Write to pipe2
/// pipe2.write(b"hello")?;
///
/// // Read from pipe1
/// let mut buf = vec![0; 5];
/// let n = pipe1.read(&mut buf)?;
/// assert_eq!(&buf[..n], b"hello");
/// ```
///
/// # Implementation Details
/// - The pipe uses a zero-capacity channel (`sync_channel(0)`), making all write operations
///   synchronous
/// - Reading is buffered: data is read from the channel into an internal buffer and then
///   served from there
/// - Either `tx` or `rx` will be `Some`, but never both, determining whether the pipe is
///   for reading or writing
///
/// # Thread Safety
/// - The receiver is wrapped in a `Mutex` to ensure thread-safe access
/// - The sender is naturally thread-safe through `SyncSender`
///
/// # Resource Management
/// The pipe automatically closes when dropped, ensuring proper cleanup of system resources.
///
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
                        #[cfg(feature = "enable-logging")]
                        log::info!("Pipe Read: channel is closed");
                        return Err(Error::new(ErrorKind::Other, "channel is closed"));
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
        match self.tx.as_mut().unwrap().send(buf.to_vec()) {
            Ok(_) => {
                #[cfg(feature = "enable-logging")]
                log::info!("Pipe Write: write {} bytes", buf.len());
                Ok(buf.len())
            }
            Err(e) => {
                #[cfg(feature = "enable-logging")]
                log::error!("Pipe Write: channel is closed {:?}", e);
                drop(e);
                Err(Error::new(ErrorKind::Other, "channel is closed"))
            }
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(has_asm)]
fn ckb_vm_entry(
    code: Bytes,
    args: Vec<Bytes>,
    read_pipe: Pipe,
    write_pipe: Pipe,
) -> Result<(), Box<dyn std::error::Error>> {
    let asm_core = ckb_vm::machine::asm::AsmCoreMachine::new(
        ckb_vm::ISA_IMC | ckb_vm::ISA_B | ckb_vm::ISA_MOP,
        ckb_vm::machine::VERSION2,
        u64::MAX,
    );
    let core = ckb_vm::DefaultMachineBuilder::new(asm_core)
        .instruction_cycle_func(Box::new(estimate_cycles))
        .syscall(Box::new(DebugSyscall {}))
        .syscall(Box::new(ReadSyscall::new(read_pipe)))
        .syscall(Box::new(WriteSyscall::new(write_pipe)))
        .syscall(Box::new(InheritedFdSyscall {}))
        .syscall(Box::new(CloseSyscall {}))
        .syscall(Box::new(DebugSyscall {}))
        .build();
    let mut machine = ckb_vm::machine::asm::AsmMachine::new(core);
    machine.load_program(&code, &args)?;
    let _exit = machine.run();
    let _cycles = machine.machine.cycles();
    Ok(())
}

#[cfg(not(has_asm))]
fn ckb_vm_entry(
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
        .instruction_cycle_func(Box::new(estimate_cycles))
        .syscall(Box::new(DebugSyscall {}))
        .syscall(Box::new(ReadSyscall::new(read_pipe)))
        .syscall(Box::new(WriteSyscall::new(write_pipe)))
        .syscall(Box::new(InheritedFdSyscall {}))
        .syscall(Box::new(CloseSyscall {}))
        .build();
    machine.load_program(&code, &args)?;
    let _exit = machine.run();
    let _cycles = machine.cycles();
    Ok(())
}
/// Spawns a new CKB-VM instance running the provided script binary in a
/// separate thread with bidirectional communication channels.
///
/// This function creates two pipes for bidirectional communication between the
/// native code and the CKB-VM instance:
/// - One pipe for sending data from native code to CKB-VM
/// - One pipe for receiving data from CKB-VM to native code
///
/// # Arguments
///
/// * `script_binary` - A byte slice containing the RISC-V binary to be executed
///   in CKB-VM
/// * `args` - A slice of string arguments to be passed to the script binary
///
/// # Returns
///
/// Returns a `Result` containing a tuple of two `Pipe`s on success:
/// - The first `Pipe` is for reading data from the CKB-VM instance
/// - The second `Pipe` is for writing data to the CKB-VM instance
///
/// # Errors
///
/// Returns a boxed error if the thread creation or VM initialization fails.
///
/// # Example
///
/// ```rust,ignore
/// let binary = include_bytes!("path/to/binary");
/// let args = &["arg1", "arg2"];
///
/// let (read_pipe, write_pipe) = spawn_server(binary, args)?;
/// // Now you can use read_pipe to receive data from the VM
/// // and write_pipe to send data to the VM
/// ```
/// # Note
/// The VM thread will be terminated when either:
/// - Both `read_pipe` and `write_pipe` are dropped, causing the communication channels to close
/// - The VM execution completes naturally
/// - An unrecoverable error occurs in the VM
pub fn spawn_server(
    script_binary: &[u8],
    args: &[&str],
) -> Result<(Pipe, Pipe), Box<dyn std::error::Error>> {
    // channel: ckb-vm -> native
    let (read_pipe1, write_pipe1) = Pipe::new_pair();
    // channel: native -> ckb-vm
    let (read_pipe2, write_pipe2) = Pipe::new_pair();

    let code = Bytes::copy_from_slice(script_binary);
    let args = args
        .iter()
        .map(|s| Bytes::copy_from_slice(s.as_bytes()))
        .collect();
    std::thread::spawn(move || {
        let _ = ckb_vm_entry(code, args, read_pipe2, write_pipe1);
    });
    Ok((read_pipe1, write_pipe2))
}
