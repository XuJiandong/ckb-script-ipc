use ckb_std::env::argv;
use ckb_std::logger;

use crate::client_entry;
use crate::error::Error;
use crate::server_entry;

pub fn entry() -> Result<(), Error> {
    drop(logger::init());
    let argv = argv();
    if argv.is_empty() {
        client_entry::client_entry()?;
    } else {
        server_entry::server_entry()?;
    }
    Ok(())
}
