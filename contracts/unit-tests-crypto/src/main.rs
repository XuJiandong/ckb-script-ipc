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
use ckb_crypto_service::{CkbCryptoClient, HasherType};
use ckb_script_ipc_common::pipe::Pipe;
use ckb_std::log::{error, info};

use ckb_script_ipc_common::spawn::spawn_cell_server;

struct CryptoInfo {
    cmd: Cmd,
    crypto_cli: CkbCryptoClient<Pipe, Pipe>,
    args: Vec<u8>,
    witness: Vec<u8>,
}

impl CryptoInfo {
    fn new() -> Self {
        let args: Vec<u8> = ckb_std::high_level::load_script()
            .unwrap()
            .args()
            .into_iter()
            .map(|f| f.into())
            .collect();

        let cmd = Cmd::from(args[0]);
        let args = args[1..].to_vec();
        let witness =
            ckb_std::high_level::load_witness_args(0, ckb_std::ckb_constants::Source::GroupInput)
                .expect("load groupinput 0 witness")
                .lock()
                .to_opt()
                .expect("witness lock")
                .raw_data()
                .to_vec();

        let (read_pipe, write_pipe) = spawn_cell_server(
            &args[0..32],
            ckb_std::ckb_types::core::ScriptHashType::Data2,
            &[CString::new("demo").unwrap().as_ref()],
        )
        .unwrap();
        let args = args[32..].to_vec();

        let crypto_cli = CkbCryptoClient::new(read_pipe, write_pipe);
        Self {
            cmd,
            crypto_cli,
            args,
            witness,
        }
    }
}

fn unit_test_ckb_blake2b(crypto_info: CryptoInfo) -> i8 {
    let mut crypto_cli = crypto_info.crypto_cli;

    let ctx = crypto_cli
        .hasher_new(HasherType::CkbBlake2b)
        .expect("init ckb blake2b");
    crypto_cli
        .hasher_update(ctx.clone(), crypto_info.witness.clone())
        .expect("update ckb blake2b");
    let hash = crypto_cli
        .hasher_finalize(ctx)
        .expect("ckb blake2b finallize");

    if hash.as_slice() != crypto_info.args.as_slice() {
        error!(
            "check ckb blake2b error: \n0: {:02x?} \n1: {:02x?}",
            hash, crypto_info.args
        );
        info!(
            "witness({}): {:02x?}",
            crypto_info.witness.len(),
            crypto_info.witness
        );
        1
    } else {
        info!("check ckb blake2b success");
        0
    }
}
fn unit_test_blake2b(crypto_info: CryptoInfo) -> i8 {
    let mut crypto_cli = crypto_info.crypto_cli;

    let ctx = crypto_cli
        .hasher_new(HasherType::Blake2b)
        .expect("init def black2b");
    crypto_cli
        .hasher_update(ctx.clone(), crypto_info.witness.clone())
        .expect("update def blake2b");
    let hash = crypto_cli
        .hasher_finalize(ctx)
        .expect("def blake2b finallize");

    if hash.as_slice() != crypto_info.args.as_slice() {
        error!(
            "check def blake2b error: \n0: {:02x?} \n1: {:02x?}",
            hash, crypto_info.args
        );
        info!(
            "witness({}): {:02x?}",
            crypto_info.witness.len(),
            crypto_info.witness
        );
        1
    } else {
        info!("check def blake2b success");
        0
    }
}

fn unit_test_sha256(crypto_info: CryptoInfo) -> i8 {
    let mut crypto_cli = crypto_info.crypto_cli;

    let ctx = crypto_cli
        .hasher_new(HasherType::Sha256)
        .expect("init sha256");
    crypto_cli
        .hasher_update(ctx.clone(), crypto_info.witness.clone())
        .expect("update sha256");
    let hash = crypto_cli
        .hasher_finalize(ctx)
        .expect("sha256 finallize");
    if hash.as_slice() != crypto_info.args.as_slice() {
        error!(
            "check sha256 error: \n0: {:02x?} \n1: {:02x?}",
            hash, crypto_info.args
        );
        info!(
            "witness({}): {:02x?}",
            crypto_info.witness.len(),
            crypto_info.witness
        );
        1
    } else {
        info!("check sha256 success");
        0
    }
}

fn unit_test_ripemd160(crypto_info: CryptoInfo) -> i8 {
    let mut crypto_cli = crypto_info.crypto_cli;

    let ctx = crypto_cli
        .hasher_new(HasherType::Ripemd160)
        .expect("init ripemd160");
    crypto_cli
        .hasher_update(ctx.clone(), crypto_info.witness.clone())
        .expect("update ripemd160");
    let hash = crypto_cli
        .hasher_finalize(ctx)
        .expect("ripemd160 finallize");
    if hash.as_slice() != crypto_info.args.as_slice() {
        error!(
            "check ripemd160 error: \n0: {:02x?} \n1: {:02x?}",
            hash, crypto_info.args
        );
        info!(
            "witness({}): {:02x?}",
            crypto_info.witness.len(),
            crypto_info.witness
        );
        1
    } else {
        info!("check ripemd160 success");
        0
    }
}

pub fn program_entry() -> i8 {
    drop(ckb_std::logger::init());
    info!("unit-tests-crypto started");

    let info = CryptoInfo::new();

    match info.cmd {
        Cmd::CkbBlake2b => unit_test_ckb_blake2b(info),
        Cmd::Blake2b => unit_test_blake2b(info),
        Cmd::Sha256 => unit_test_sha256(info),
        Cmd::Ripemd160 => unit_test_ripemd160(info),
    }
}