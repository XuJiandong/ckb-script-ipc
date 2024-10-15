use crate::error::Error;
use alloc::{ffi::CString, string::String};
use ckb_script_ipc_common::spawn::spawn_server;
use ckb_std::{ckb_constants::Source, log::info};

// IPC definition
#[ckb_script_ipc::service]
trait World {
    fn hello(name: String) -> Result<String, u64>;
}

pub fn client_entry() -> Result<(), Error> {
    info!("client started");

    // server can be spawned by any process which wants to start it.
    // here it is invoked by client
    let (read_pipe, write_pipe) = spawn_server(
        0,
        Source::CellDep,
        &[CString::new("demo").unwrap().as_ref()],
    )
    .map_err(|_| Error::CkbSysError)?;

    let mut client = WorldClient::new(read_pipe.into(), write_pipe.into());
    let ret = client.hello("world".into()).unwrap();
    info!("IPC response: {:?}", ret);
    Ok(())
}
