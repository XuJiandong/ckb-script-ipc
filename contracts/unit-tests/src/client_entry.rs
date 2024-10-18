use alloc::{collections::BTreeMap, ffi::CString, string::ToString, vec};
use ckb_script_ipc_common::spawn::spawn_server;
use ckb_std::{ckb_constants::Source, log::info};

use crate::{
    def::{Struct0, Struct1, UnitTestsClient},
    error::Error,
};

pub fn client_entry() -> Result<(), Error> {
    let (read_pipe, write_pipe) = spawn_server(
        0,
        Source::CellDep,
        &[CString::new("demo").unwrap().as_ref()],
    )
    .map_err(|_| Error::CkbSysError)?;

    // new client
    let mut client = UnitTestsClient::new(read_pipe, write_pipe);
    client.test_primitive_types(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, true);
    info!("test_primitive_types success");

    // Test Vec
    client.test_vec(vec![1, 2, 3, 4, 5]);
    info!("test_vec success");

    // Test BTreeMap
    let mut map = BTreeMap::new();
    map.insert("one".to_string(), 1);
    map.insert("two".to_string(), 2);
    map.insert("three".to_string(), 3);
    client.test_btree_map(map);
    info!("test_btree_map success");

    // Test Complex Types
    let complex_struct = Struct1 {
        f1: 1,
        f2: 2,
        f3: [3, 3, 3],
        f4: [[4, 4, 4, 4, 4], [5, 5, 5, 5, 5]],
        f5: vec![6, 7, 8],
        f6: "test".to_string(),
        f7: Some(9),
        f8: vec![vec![10, 11], vec![12, 13]],
        f9: Struct0 {
            f0: 14,
            f1: 15,
            f2: [16, 17, 18],
        },
    };
    client.test_complex_types(complex_struct);
    info!("test_complex_types success");

    // Test Return Types
    match client.test_return_types() {
        Ok(value) => {
            assert_eq!(value, 42);
            info!("test_return_types success with value: {}", value);
        }
        Err(err) => {
            info!("test_return_types failed with error: {}", err);
        }
    }

    Ok(())
}
