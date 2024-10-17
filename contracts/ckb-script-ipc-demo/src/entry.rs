use ckb_std::env::argv;
use ckb_std::log::info;
use ckb_std::logger;

use crate::client_entry;
use crate::error::Error;
use crate::server_entry;

pub fn entry() -> Result<(), Error> {
    drop(logger::init());
    info!("entry started");

    let argv = argv();
    // in real life project, the client_entry and server_entry will be in
    // different projects. the following code is just for demo purpose
    if argv.is_empty() {
        client_entry::client_entry()?;
    } else {
        server_entry::server_entry()?;
    }
    Ok(())
}
