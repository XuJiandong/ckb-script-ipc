use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum CryptoError {}

#[derive(Serialize, Deserialize)]
pub enum HasherType {
    CkbBlake2b,
    Blake2b,
    Sha256,
    Ripemd160,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HasherCtx(pub u64);

#[ckb_script_ipc::service]
pub trait CkbCrypto {
    fn hasher_new(hash_type: HasherType) -> Result<HasherCtx, CryptoError>;
    fn hasher_update(ctx: HasherCtx, data: Vec<u8>) -> Result<(), CryptoError>;
    fn hasher_finalize(ctx: HasherCtx) -> Result<Vec<u8>, CryptoError>;
}
