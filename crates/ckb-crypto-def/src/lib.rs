#![no_std]
extern crate alloc;

#[ckb_script_ipc::service]
pub trait CkbCrypto {
    fn ckbblake2b_init() -> Result<u64, u64>;
    fn ckbblake2b_update(ctx: u64, data: alloc::vec::Vec<u8>) -> Result<(), u64>;
    fn ckbblake2b_finalize(ctx: u64) -> Result<[u8; 32], u64>;

    fn sha256_init() -> Result<u64, u64>;
    fn sha256_update(ctx: u64, data: alloc::vec::Vec<u8>) -> Result<(), u64>;
    fn sha256_finalize(ctx: u64) -> Result<[u8; 32], u64>;
}
