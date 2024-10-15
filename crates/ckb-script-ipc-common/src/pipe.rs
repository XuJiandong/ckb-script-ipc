use crate::error::IpcError;
use crate::io::{Read, Write};
use ckb_std::syscalls::{read, write};

pub struct Pipe {
    id: u64,
}

impl Pipe {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn fd(&self) -> u64 {
        self.id
    }

    pub fn readable(&self) -> bool {
        self.id % 2 == 0
    }

    pub fn writable(&self) -> bool {
        self.id % 2 == 1
    }
}

impl From<u64> for Pipe {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

impl Read for Pipe {
    type Error = IpcError;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        match read(self.id, buf) {
            Ok(n) => Ok(n),
            Err(e) => Err(IpcError::CkbSysError(e)),
        }
    }
}

impl Write for Pipe {
    type Error = IpcError;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        match write(self.id, buf) {
            Ok(n) => Ok(n),
            Err(e) => Err(IpcError::CkbSysError(e)),
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
