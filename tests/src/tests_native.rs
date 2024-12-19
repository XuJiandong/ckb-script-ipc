use ckb_script_ipc_common::{channel::Channel, native::spawn_server};
use unit_tests_def::UnitTestsClient;

#[test]
fn test_native() {
    let script_binary = std::fs::read("../build/release/unit-tests").unwrap();
    let (read_pipe, write_pipe) = spawn_server(&script_binary, &["server_entry"]).unwrap();

    let mut client = UnitTestsClient::new(read_pipe, write_pipe);
    client.test_primitive_types(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, true);

    // new instance, test granceful shutdown
    let script_binary = std::fs::read("../build/release/unit-tests").unwrap();
    let (read_pipe, write_pipe) = spawn_server(&script_binary, &["server_entry"]).unwrap();

    let mut client = UnitTestsClient::new(read_pipe, write_pipe);

    let result = client.test_return_types();
    assert_eq!(result, Ok(42));
}

#[test]
fn test_native_json() {
    let script_binary = std::fs::read("../build/release/unit-tests").unwrap();
    let (read_pipe, write_pipe) = spawn_server(&script_binary, &["server_entry"]).unwrap();

    // directly call with json
    let json = r#"
    {"TestPrimitiveTypes":{"arg1":1,"arg2":2,"arg3":3,"arg4":4,"arg5":5,"arg6":6,"arg7":7,"arg8":8,"arg9":9,"arg10":10,"arg11":true}}
    "#;
    let mut channel = Channel::new(read_pipe, write_pipe);
    channel.send_json_request(json).unwrap();
    let response = channel.receive_json_response().unwrap();
    assert_eq!(response, "{\"TestPrimitiveTypes\":null}");
}

#[test]
fn test_native_stress() {
    let script_binary = std::fs::read("../build/release/unit-tests").unwrap();
    let (read_pipe, write_pipe) = spawn_server(&script_binary, &["server_entry"]).unwrap();

    let mut client = UnitTestsClient::new(read_pipe, write_pipe);
    for _ in 0..100 {
        client.test_primitive_types(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, true);
    }
    let input = vec![0; 16 * 1024];
    let output = client.test_large_input_output(input.clone());
    assert_eq!(output, input.into_iter().map(|x| x + 1).collect::<Vec<_>>());
}
