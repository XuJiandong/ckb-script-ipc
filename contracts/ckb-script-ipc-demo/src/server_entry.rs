use crate::def::World;
use crate::error::Error;
use alloc::{format, string::String};
use ckb_script_ipc_common::spawn::run_server;
use ckb_std::log::info;

struct WorldServer;

impl WorldServer {
    fn new() -> Self {
        WorldServer
    }
}

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
    let world = WorldServer::new();
    run_server(world.server()).map_err(|_| Error::ServerError)
}
