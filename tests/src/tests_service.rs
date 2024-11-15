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
fn test_service_ckb_blake2b() {
    let buffer = [0u8; 256];
    let hash = ckb_testtool::ckb_hash::blake2b_256(&buffer);
    run_service_test(Cmd::CkbBlake2b, hash.to_vec(), buffer.to_vec())
}

#[test]
fn test_service_sha256() {
    let buffer = [0u8; 256];

    use sha2::{Digest, Sha256};
    let mut ctx = Sha256::new();
    ctx.update(&buffer);
    let hash = ctx.finalize().to_vec();

    run_service_test(Cmd::Sha256, hash, buffer.to_vec())
}
