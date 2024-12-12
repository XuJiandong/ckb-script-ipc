use ckb_script_ipc_common::native::spawn_server;
use unit_tests_def::UnitTestsClient;

#[test]
fn test_native() {
    env_logger::init();
    let script_binary = std::fs::read("../build/release/unit-tests").unwrap();
    let (read_pipe, write_pipe) = spawn_server(&script_binary, &["server_entry"]).unwrap();

    let mut client = UnitTestsClient::new(read_pipe, write_pipe);
    client.test_primitive_types(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, true);

    let result = client.test_return_types();
    assert_eq!(result, Ok(42));
}
