use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{DepType, TransactionBuilder},
    packed::*,
    prelude::*,
};
use ckb_testtool::context::Context;

#[test]
fn test_service_blake2b() {
    // deploy contract
    let mut context = Context::default();

    let service_outpoint = context.deploy_cell_by_name("ckb-crypto-service");

    let out_point = context.deploy_cell_by_name("unit-tests-crypto");
    let lock_args = vec![0];

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
