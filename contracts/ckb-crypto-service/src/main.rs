#![no_std]
#![cfg_attr(not(test), no_main)]

#[cfg(test)]
extern crate alloc;

#[cfg(not(test))]
use ckb_std::default_alloc;
#[cfg(not(test))]
ckb_std::entry!(program_entry);
#[cfg(not(test))]
default_alloc!();

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use ckb_crypto_def::CkbCrypto;
use ckb_script_ipc_common::spawn::run_server;
use ckb_std::log::{error, info};

trait Hasher {
    fn update(&mut self, data: &[u8]);
    fn finalize(&mut self) -> Vec<u8>;
}

struct Sha256Hasher {
    ctx: Option<sha2::Sha256>,
}
impl Hasher for Sha256Hasher {
    fn update(&mut self, data: &[u8]) {
        use sha2::Digest;
        self.ctx.as_mut().unwrap().update(data);
    }
    fn finalize(&mut self) -> Vec<u8> {
        use sha2::Digest;
        let ctx = self.ctx.take().unwrap();
        ctx.finalize().to_vec()
    }
}

struct CkbBlake2b {
    ctx: Option<blake2b_ref::Blake2b>,
}
impl Hasher for CkbBlake2b {
    fn update(&mut self, data: &[u8]) {
        self.ctx.as_mut().unwrap().update(data);
    }
    fn finalize(&mut self) -> Vec<u8> {
        let ctx = self.ctx.take().unwrap();
        let mut buf = [0u8; 32];
        ctx.finalize(&mut buf);
        buf.to_vec()
    }
}

struct CryptoServer {
    hashers: BTreeMap<u64, Box<dyn Hasher>>,
    hasher_count: u64,
}

impl CryptoServer {
    fn new() -> Self {
        Self {
            hashers: Default::default(),
            hasher_count: 0,
        }
    }
    fn insert_haser(&mut self, hasher: Box<dyn Hasher>) -> u64 {
        let ctx_id = self.hasher_count;
        self.hasher_count += 1;
        self.hashers.insert(ctx_id, hasher);
        ctx_id
    }
}

impl CkbCrypto for CryptoServer {
    // method implementation
    fn ckbblake2b_init(&mut self) -> Result<u64, u64> {
        const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";

        let hasher = Box::new(CkbBlake2b {
            ctx: Some(
                blake2b_ref::Blake2bBuilder::new(32)
                    .personal(CKB_HASH_PERSONALIZATION)
                    .build(),
            ),
        });

        Ok(self.insert_haser(hasher))
    }
    fn ckbblake2b_update(&mut self, ctx: u64, data: alloc::vec::Vec<u8>) -> Result<(), u64> {
        let hasher = self.hashers.get_mut(&ctx).expect("find ctx");
        hasher.update(&data);
        Ok(())
    }
    fn ckbblake2b_finalize(&mut self, ctx: u64) -> Result<[u8; 32], u64> {
        let mut hasher = self.hashers.remove(&ctx).expect("find ctx");
        let buf = hasher.finalize();
        Ok(buf.try_into().unwrap())
    }

    fn sha256_init(&mut self) -> Result<u64, u64> {
        use sha2::{Digest, Sha256};
        let hasher = Box::new(Sha256Hasher {
            ctx: Some(Sha256::new()),
        });

        Ok(self.insert_haser(hasher))
    }
    fn sha256_update(&mut self, ctx: u64, data: alloc::vec::Vec<u8>) -> Result<(), u64> {
        let hasher = self.hashers.get_mut(&ctx).expect("find ctx");
        hasher.update(&data);
        Ok(())
    }
    fn sha256_finalize(&mut self, ctx: u64) -> Result<[u8; 32], u64> {
        let mut hasher = self.hashers.remove(&ctx).expect("find ctx");
        let buf = hasher.finalize();
        Ok(buf.try_into().unwrap())
    }
}

pub fn program_entry() -> i8 {
    drop(ckb_std::logger::init());

    info!("server started");
    let world = CryptoServer::new();
    let err = run_server(world.server());

    if err.is_ok() {
        0
    } else {
        error!("Server failed: {:?}", err.unwrap_err());
        1
    }
}
