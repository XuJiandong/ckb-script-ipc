use crate::error::IpcError;
use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::core::ScriptHashType,
    high_level::spawn_cell,
    syscalls::{self, pipe},
};
use core::ffi::CStr;

pub fn spawn_server(index: usize, source: Source, argv: &[&CStr]) -> Result<(u64, u64), IpcError> {
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
    Ok((r1, w2))
}

pub fn spawn_cell_server(
    code_hash: &[u8],
    hash_type: ScriptHashType,
    argv: &[&CStr],
) -> Result<(u64, u64), IpcError> {
    let (r1, w1) = pipe().map_err(IpcError::CkbSysError)?;
    let (r2, w2) = pipe().map_err(IpcError::CkbSysError)?;
    let inherited_fds = &[r2, w1];

    spawn_cell(code_hash, hash_type, argv, inherited_fds).map_err(IpcError::CkbSysError)?;
    Ok((r1, w2))
}
