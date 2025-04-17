use crate::Loader;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*};
use ckb_testtool::context::Context;

fn test_c_impl(client_path: &str, server_path: &str) {
    let mut context = Context::default();
    let server_bin: Bytes = Loader::default().load_binary(server_path);
    let server_out_point = context.deploy_cell(server_bin);

    let client_bin: Bytes = Loader::default().load_binary(client_path);
    let client_out_point = context.deploy_cell(client_bin);

    // prepare scripts
    let lock_script = context
        .build_script(&client_out_point, Bytes::default())
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
    // make the server on-chain script as the first cell_dep
    let tx = TransactionBuilder::default()
        .cell_deps(vec![CellDep::new_builder()
            .out_point(server_out_point)
            .build()])
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_c_impl_examples() {
    test_c_impl(
        "../../c/build/examples/client",
        "../../c/build/examples/server",
    );
}

#[test]
fn test_c_impl_tests() {
    test_c_impl("../../c/build/tests/client", "../../c/build/tests/server");
}
