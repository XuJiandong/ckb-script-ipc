#![no_std]
extern crate alloc;
pub mod channel;
pub mod error;
pub mod io;
pub mod ipc;
#[cfg(feature = "std")]
pub mod native;
pub mod packet;
pub mod pipe;
pub mod spawn;
pub mod vlq;
