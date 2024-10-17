use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use ckb_script_ipc_common::channel::Channel;
use ckb_std::high_level::inherited_fds;

use crate::{
    def::{Struct0, Struct1, UnitTests},
    error::Error,
};

struct UnitTestsServer;

impl UnitTests for UnitTestsServer {
    fn test_primitive_types(
        &mut self,
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
    ) {
        assert_eq!(arg1, 1);
        assert_eq!(arg2, 2);
        assert_eq!(arg3, 3);
        assert_eq!(arg4, 4);
        assert_eq!(arg5, 5);
        assert_eq!(arg6, 6);
        assert_eq!(arg7, 7);
        assert_eq!(arg8, 8);
        assert_eq!(arg9, 9);
        assert_eq!(arg10, 10);
        assert_eq!(arg11, true);
    }

    fn test_vec(&mut self, vec: Vec<i32>) {
        assert_eq!(vec, vec![1, 2, 3, 4, 5]);
    }

    fn test_btree_map(&mut self, map: BTreeMap<String, i32>) {
        let mut expected_map = BTreeMap::new();
        expected_map.insert("one".to_string(), 1);
        expected_map.insert("two".to_string(), 2);
        expected_map.insert("three".to_string(), 3);
        assert_eq!(map, expected_map);
    }

    fn test_complex_types(&mut self, arg1: Struct1) {
        let expected = Struct1 {
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
        assert_eq!(arg1, expected);
    }

    fn test_return_types(&mut self) -> Result<u32, String> {
        let success = true;
        if success {
            Ok(42)
        } else {
            Err("An error occurred".to_string())
        }
    }
}

pub fn server_entry() -> Result<(), Error> {
    let fds = inherited_fds();
    assert_eq!(fds.len(), 2);
    let channel = Channel::new(fds[0].into(), fds[1].into());
    channel
        .execute(&mut UnitTestsServer.server())
        .map_err(|_| Error::ServerError)?;
    Ok(())
}
