#[ckb_script_ipc::service]
pub trait World {
    fn hello(name: String) -> Result<String, u64>;
}

// the following code is written by users
struct WorldServer;

impl World for WorldServer {
    fn hello(&mut self, name: String) -> Result<String, u64> {
        if name == "error" {
            Err(1)
        } else {
            Ok(format!("hello, {}", name))
        }
    }
}

#[test]
fn test_generate() {
    let _ = WorldServer;
    let reader: Pipe = 0u64.into();
    let writer: Pipe = 1u64.into();
    let _ = WorldClient::new(reader, writer);
}

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::LinkedList;
use ckb_script_ipc_common::pipe::Pipe;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Struct1 {
    pub f1: u8,
    pub f2: u16,
    pub f3: [u8; 3],
    pub f4: [[u8; 5]; 2],
    pub f5: Vec<u8>,
    pub f6: String,
    pub f7: Option<u32>,
    pub f8: Vec<Vec<u8>>,
    pub f9: LinkedList<[u8; 3]>,
    pub f11: BTreeMap<u32, String>,
}

#[ckb_script_ipc::service]
pub trait SerdeMolecule {
    fn f1_func(bytes: Vec<u8>, name: String, tests: [u8; 20]) -> Result<String, u64>;
    fn f2_func(s1: Struct1) -> Result<String, u64>;
}

#[ckb_script_ipc::service]
pub trait ConditionalService {
    #[cfg(any())]
    fn disabled(value: UnavailableType) -> UnavailableType;

    #[cfg(all())]
    fn enabled(value: u32) -> u32;
}

struct ConditionalServer;

impl ConditionalService for ConditionalServer {
    fn enabled(&mut self, value: u32) -> u32 {
        value + 1
    }
}

#[test]
fn test_conditionally_compiled_method() {
    use ckb_script_ipc_common::ipc::Serve;

    let request =
        serde_json::from_str::<ConditionalServiceRequest>(r#"{"Enabled":{"value":41}}"#).unwrap();
    let response = ConditionalServer.server().serve(request).unwrap();
    let ConditionalServiceResponse::Enabled(value) = response;
    assert_eq!(value, 42);
    assert_eq!(
        serde_json::to_string(&response).unwrap(),
        r#"{"Enabled":42}"#
    );
}
