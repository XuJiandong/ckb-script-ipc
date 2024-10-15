use crate::error::Error;
use alloc::{format, string::String};
use ckb_script_ipc_common::channel::Channel;
use ckb_std::{high_level::inherited_fds, log::info};

// IPC definition
#[ckb_script_ipc::service]
trait World {
    fn hello(name: String) -> Result<String, u64>;
}

struct WorldServer;

impl World for WorldServer {
    // method implementation
    fn hello(&mut self, name: String) -> Result<String, u64> {
        if name == "error" {
            Err(1)
        } else {
            Ok(format!("hello, {}", name))
        }
    }
}

pub fn server_entry() -> Result<(), Error> {
    info!("server started");
    // new the channel
    let fds = inherited_fds();
    assert_eq!(fds.len(), 2);
    let channel = Channel::new(fds[0].into(), fds[1].into());
    // execute the server
    channel
        .execute(&mut WorldServer.server())
        .map_err(|_| Error::ServerError)?;
    Ok(())
}
