#![cfg_attr(not(any(feature = "native-simulator", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "native-simulator", test))]
extern crate alloc;

#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "native-simulator", test)))]
ckb_std::default_alloc!();

mod unit_tests_crypto_def;
use unit_tests_crypto_def::Cmd;

use alloc::ffi::CString;
use alloc::vec::Vec;
use ckb_crypto_def::CkbCryptoClient;
use ckb_script_ipc_common::{pipe::Pipe, spawn::spawn_server};
use ckb_std::log::info;

fn unit_test_blake2b(cmd: CkbCryptoClient<Pipe, Pipe>) -> i8 {
    let mut cmd = cmd;
    let ctx = cmd.sha256_init().expect("init black2b");
    cmd.sha256_update(ctx, alloc::vec![0u8; 32])
        .expect("update blake2b");

    let hash = cmd.sha256_finalize(ctx).expect("msg");
    info!("blake2b hash: {:02x?}", hash);

    0
}

pub fn program_entry() -> i8 {
    drop(ckb_std::logger::init());
    info!("unit-tests-crypto started");

    let args: Vec<u8> = ckb_std::high_level::load_script()
        .unwrap()
        .args()
        .into_iter()
        .map(|f| f.into())
        .collect();

    let (read_pipe, write_pipe) = spawn_server(
        0,
        ckb_std::ckb_constants::Source::CellDep,
        &[CString::new("demo").unwrap().as_ref()],
    )
    .unwrap();

    let cmd = CkbCryptoClient::new(read_pipe, write_pipe);

    match Cmd::from(args[0]) {
        Cmd::Blake2b => unit_test_blake2b(cmd),
    }
}
