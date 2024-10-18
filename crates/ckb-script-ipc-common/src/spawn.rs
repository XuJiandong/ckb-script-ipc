use crate::{channel::Channel, error::IpcError, ipc::Serve, pipe::Pipe};
use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::core::ScriptHashType,
    high_level::{inherited_fds, spawn_cell},
    syscalls::{self, pipe},
};
use core::ffi::CStr;
use serde::{Deserialize, Serialize};
/// Spawns a new server process and sets up pipes.
///
/// This function creates two pairs of pipes for communication between the parent and child processes.
/// It then spawns a new process using the specified index and source, passing the provided arguments
/// to the new process. The function returns the read and write file descriptors for the parent process
/// to communicate with the child process.
///
/// # Arguments
///
/// * `index` - The index of the cell to spawn.
/// * `source` - The source of the cell (e.g., `Source::CellDep`).
/// * `argv` - A slice of C strings representing the arguments to pass to the new process.
///
/// # Returns
///
/// A `Result` containing a tuple of two `Pipe` representing the read and write file descriptors
/// for the parent process, or an `IpcError` if an error occurs.
///
/// # Errors
///
/// This function returns an `IpcError` if any of the following syscalls fail:
/// * `pipe` - If creating a pipe fails.
/// * `spawn` - If spawning the new process fails.
///
/// # Example
///
/// ```rust,ignore
/// use ckb_script_ipc_common::spawn::spawn_server;
///
/// let (read_pipe, write_pipe) = spawn_server(
///     0,
///     Source::CellDep,
///     &[CString::new("demo").unwrap().as_ref()],
/// ).expect("Failed to spawn server");
/// ```
pub fn spawn_server(
    index: usize,
    source: Source,
    argv: &[&CStr],
) -> Result<(Pipe, Pipe), IpcError> {
    let (r1, w1) = pipe().map_err(IpcError::CkbSysError)?;
    let (r2, w2) = pipe().map_err(IpcError::CkbSysError)?;
    let inherited_fds = &[r2, w1];

    let argc = argv.len();
    let mut process_id: u64 = 0;
    let argv_ptr: Vec<*const i8> = argv.iter().map(|&e| e.as_ptr()).collect();
    let mut spgs = syscalls::SpawnArgs {
        argc: argc as u64,
        argv: argv_ptr.as_ptr(),
        process_id: &mut process_id,
        inherited_fds: inherited_fds.as_ptr(),
    };
    syscalls::spawn(index, source, 0, 0, &mut spgs).map_err(IpcError::CkbSysError)?;
    Ok((r1.into(), w2.into()))
}
/// Spawns a new server process using the provided code hash and hash type. This function is similar
/// to `spawn_server`, but it uses a specific cell identified by the `code_hash` and `hash_type` to
/// spawn the new process. The function returns the read and write file descriptors for the parent
/// process to communicate with the child process.
///
/// # Arguments
///
/// * `code_hash` - A byte slice representing the code hash of the cell to spawn.
/// * `hash_type` - The hash type of the cell (e.g., `ScriptHashType::Type`).
/// * `argv` - A slice of C strings representing the arguments to pass to the new process.
///
/// # Returns
///
/// A `Result` containing a tuple of two `Pipe` representing the read and write file descriptors
/// for the parent process, or an `IpcError` if an error occurs.
///
/// # Errors
///
/// This function returns an `IpcError` if any of the following syscalls fail:
/// * `pipe` - If creating a pipe fails.
/// * `spawn_cell` - If spawning the new process using the cell fails.
///
/// # Example
///
/// ```rust,ignore
/// use ckb_script_ipc_common::spawn::spawn_cell_server;
///
/// let (read_pipe, write_pipe) = spawn_cell_server(
///     code_hash,
///     hash_type,
///     &[CString::new("demo").unwrap().as_ref()],
/// ).expect("Failed to spawn cell server");
/// ```
pub fn spawn_cell_server(
    code_hash: &[u8],
    hash_type: ScriptHashType,
    argv: &[&CStr],
) -> Result<(Pipe, Pipe), IpcError> {
    let (r1, w1) = pipe().map_err(IpcError::CkbSysError)?;
    let (r2, w2) = pipe().map_err(IpcError::CkbSysError)?;
    let inherited_fds = &[r2, w1];

    spawn_cell(code_hash, hash_type, argv, inherited_fds).map_err(IpcError::CkbSysError)?;
    Ok((r1.into(), w2.into()))
}
/// Runs the server with the provided service implementation. This function listens for incoming
/// requests, processes them using the provided service, and sends back the responses. It uses
/// the inherited file descriptors for communication.
///
/// # Arguments
///
/// * `serve` - A mutable reference to the service implementation that handles the requests and
///   generates the responses. The service must implement the `Serve` trait with the appropriate
///   request and response types.
///
/// # Type Parameters
///
/// * `Req` - The type of the request messages. It must implement `Serialize` and `Deserialize`.
/// * `Resp` - The type of the response messages. It must implement `Serialize` and `Deserialize`.
/// * `S` - The type of the service implementation. It must implement the `Serve` trait with
///   `Req` as the request type and `Resp` as the response type.
///
/// # Returns
///
/// A `Result` indicating the success or failure of the server execution. If the server runs
/// successfully, it never returns. If an error occurs, it returns an `IpcError`.
///
/// # Errors
///
/// This function returns an `IpcError` if any of the following conditions occur:
/// * The inherited file descriptors are not exactly two.
/// * An error occurs during the execution of the channel.
pub fn run_server<Req, Resp, S>(mut serve: S) -> Result<(), IpcError>
where
    Req: Serialize + for<'de> Deserialize<'de>,
    Resp: Serialize + for<'de> Deserialize<'de>,
    S: Serve<Req = Req, Resp = Resp>,
{
    let fds = inherited_fds();
    assert_eq!(fds.len(), 2);

    let reader: Pipe = fds[0].into();
    let writer: Pipe = fds[1].into();
    let channel = Channel::new(reader, writer);
    channel.execute(&mut serve)
}
