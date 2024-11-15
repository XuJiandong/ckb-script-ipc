use ckb_rust_std::io::{Error, ErrorKind};
use ckb_rust_std::io::{Read, Write};
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match read(self.id, buf) {
            Ok(n) => Ok(n),
            Err(_e) => Err(Error::Simple(ErrorKind::Other)),
        }
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        match write(self.id, buf) {
            Ok(n) => Ok(n),
            Err(_e) => Err(Error::Simple(ErrorKind::Other)),
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
