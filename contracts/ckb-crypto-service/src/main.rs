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

mod crypto_interface;

use crate::crypto_interface::{CkbCrypto, CryptoError, HasherCtx, HasherType};
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use ckb_script_ipc_common::spawn::run_server;
use ckb_std::log::{error, info};

trait Hasher {
    fn update(&mut self, data: &[u8]);
    fn finalize(&mut self) -> Vec<u8>;
}

struct Blake2b {
    ctx: Option<blake2b_ref::Blake2b>,
}
impl Hasher for Blake2b {
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

struct Ripemd160Hasher {
    ctx: Option<ripemd::digest::core_api::CoreWrapper<ripemd::Ripemd160Core>>,
}
impl Hasher for Ripemd160Hasher {
    fn update(&mut self, data: &[u8]) {
        use ripemd::Digest;
        self.ctx.as_mut().unwrap().update(data);
    }
    fn finalize(&mut self) -> Vec<u8> {
        use ripemd::Digest;
        let ctx = self.ctx.take().unwrap();
        ctx.finalize().to_vec()
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
}

impl CkbCrypto for CryptoServer {
    fn hasher_new(&mut self, hash_type: HasherType) -> Result<HasherCtx, CryptoError> {
        const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";

        let hash: Box<dyn Hasher> = match hash_type {
            HasherType::CkbBlake2b => Box::new(Blake2b {
                ctx: Some(
                    blake2b_ref::Blake2bBuilder::new(32)
                        .personal(CKB_HASH_PERSONALIZATION)
                        .build(),
                ),
            }),
            HasherType::Blake2b => Box::new(Blake2b {
                ctx: Some(blake2b_ref::Blake2bBuilder::new(32).build()),
            }),
            HasherType::Sha256 => {
                use sha2::{Digest, Sha256};
                Box::new(Sha256Hasher {
                    ctx: Some(Sha256::new()),
                })
            }
            HasherType::Ripemd160 => {
                use ripemd::{Digest, Ripemd160};
                Box::new(Ripemd160Hasher {
                    ctx: Some(Ripemd160::new()),
                })
            }
        };

        let id = self.hasher_count;
        self.hasher_count += 1;
        self.hashers.insert(id, hash);
        Ok(HasherCtx(id))
    }
    fn hasher_update(&mut self, ctx: HasherCtx, data: Vec<u8>) -> Result<(), CryptoError> {
        let hasher = self.hashers.get_mut(&ctx.0).expect("find ctx");
        hasher.update(&data);
        Ok(())
    }
    fn hasher_finalize(&mut self, ctx: HasherCtx) -> Result<Vec<u8>, CryptoError> {
        let mut hasher = self.hashers.remove(&ctx.0).expect("find ctx");
        let buf = hasher.finalize();
        Ok(buf)
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
