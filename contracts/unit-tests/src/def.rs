#![allow(clippy::too_many_arguments)]

use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_molecule::{dynvec_serde, struct_serde};

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug)]
pub struct Struct0 {
    pub f0: u8,
    pub f1: u64,
    pub f2: [u8; 3],
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug)]
pub struct Struct1 {
    pub f1: u8,
    pub f2: u16,
    pub f3: [u8; 3],
    pub f4: [[u8; 5]; 2],
    pub f5: Vec<u8>,
    pub f6: String,
    pub f7: Option<u32>,
    #[serde(with = "dynvec_serde")]
    pub f8: Vec<Vec<u8>>,
    #[serde(with = "struct_serde")]
    pub f9: Struct0,
}

// IPC definition, it can be shared between client and server
#[ckb_script_ipc::service]
pub trait UnitTests {
    fn test_primitive_types(
        arg1: i8,
        arg2: u8,
        arg3: i16,
        arg4: u16,
        arg5: i32,
        arg6: u32,
        arg7: i64,
        arg8: u64,
        arg9: i128,
        arg10: u128,
        arg11: bool,
    );

    fn test_vec(vec: Vec<i32>);
    fn test_btree_map(map: BTreeMap<String, i32>);

    fn test_complex_types(arg1: Struct1);
    fn test_return_types() -> Result<u32, String>;
}
