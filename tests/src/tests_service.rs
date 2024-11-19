use crate::service_def::Cmd;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{DepType, TransactionBuilder},
    packed::*,
    prelude::*,
};
use ckb_testtool::context::Context;

fn run_service_test(cmd: Cmd, args: Vec<u8>, witness: Vec<u8>) {
    let mut context = Context::default();

    let service_outpoint = context.deploy_cell_by_name("ckb-crypto-service");

    let out_point = context.deploy_cell_by_name("unit-tests-crypto");
    let lock_args = [
        vec![cmd.into()],
        {
            context
                .cells
                .get(&service_outpoint)
                .map(|(_, bin)| CellOutput::calc_data_hash(bin).as_bytes().to_vec())
                .unwrap()
        },
        args,
    ]
    .concat();

    let lock_script = context
        .build_script(&out_point, Bytes::from(lock_args))
        .expect("script");

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(
            WitnessArgs::new_builder()
                .lock(Some(ckb_testtool::ckb_types::bytes::Bytes::from(witness)).pack())
                .build()
                .as_bytes()
                .pack(),
        )
        .cell_dep(
            CellDep::new_builder()
                .out_point(service_outpoint)
                .dep_type(DepType::Code.into())
                .build(),
        )
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_ckb_blake2b() {
    let buffer = [0u8; 256];
    let hash = ckb_testtool::ckb_hash::blake2b_256(&buffer);
    run_service_test(Cmd::CkbBlake2b, hash.to_vec(), buffer.to_vec())
}

#[test]
fn test_def_blake2b() {
    let buffer = [0u8; 256];

    let mut ctx = blake2b_ref::Blake2bBuilder::new(32).build();
    ctx.update(&buffer);
    let mut hash = [0u8; 32];
    ctx.finalize(&mut hash);
    run_service_test(Cmd::Blake2b, hash.to_vec(), buffer.to_vec())
}

#[test]
fn test_sha256() {
    let buffer = [0u8; 256];

    use sha2::{Digest, Sha256};
    let mut ctx = Sha256::new();
    ctx.update(&buffer);
    let hash = ctx.finalize().to_vec();

    run_service_test(Cmd::Sha256, hash, buffer.to_vec())
}

#[test]
fn test_ripemd160() {
    let buffer = [0u8; 256];

    use ripemd::{Digest, Ripemd160};
    let mut ctx = Ripemd160::new();
    ctx.update(&buffer);
    let hash = ctx.finalize().to_vec();

    run_service_test(Cmd::Ripemd160, hash, buffer.to_vec())
}

#[test]
fn test_secp256k1_recovery() {
    // recv
    let prehash = [0u8; 32];

    let prikey_byte: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ];
    let prikey = k256::ecdsa::SigningKey::from_slice(&prikey_byte).unwrap();
    let pubkey = prikey.verifying_key();
    let (sig, recovery_id) = prikey.sign_prehash_recoverable(&prehash).unwrap();

    let mut witness = Vec::new();
    witness.push(prehash.len() as u8);
    witness.extend_from_slice(&prehash);

    let sig = sig.to_bytes().to_vec();
    witness.push(sig.len() as u8);
    witness.extend_from_slice(&sig);

    witness.push(recovery_id.to_byte());

    let verfiy_key = pubkey.to_sec1_bytes().to_vec();

    run_service_test(Cmd::Secp256k1Recover, verfiy_key.to_vec(), witness)
}

#[test]
fn test_schnorr_verfiy() {
    use k256::schnorr::{signature::Signer, SigningKey};
    // recv
    let prehash = [0u8; 32];

    let prikey_byte: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ];
    let prikey = SigningKey::from_bytes(&prikey_byte).unwrap();
    let pubkey = prikey.verifying_key();

    let mut witness = Vec::new();
    witness.push(prehash.len() as u8);
    witness.extend_from_slice(&prehash);

    let sig = prikey.sign(&prehash).to_bytes().to_vec();
    witness.push(sig.len() as u8);
    witness.extend_from_slice(&sig);

    run_service_test(Cmd::SchnorrVerify, pubkey.to_bytes().to_vec(), witness)
}

#[test]
fn test_ed25519_verfiy() {
    use ed25519_dalek::{Signer, SigningKey};
    // recv
    let prehash = [0u8; 32];

    let prikey_byte: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ];
    let prikey = SigningKey::from_bytes(&prikey_byte);
    let pubkey = prikey.verifying_key();

    let mut witness = Vec::new();
    witness.push(prehash.len() as u8);
    witness.extend_from_slice(&prehash);

    let sig = prikey.sign(&prehash).to_vec();
    witness.push(sig.len() as u8);
    witness.extend_from_slice(&sig);

    run_service_test(Cmd::Ed25519Verfiy, pubkey.to_bytes().to_vec(), witness)
}
