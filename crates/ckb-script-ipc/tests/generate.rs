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
