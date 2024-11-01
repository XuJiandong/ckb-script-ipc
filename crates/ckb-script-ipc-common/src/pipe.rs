use ckb_core_io::Error as CoreIOError;
use ckb_core_io::{Read, Write};
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, CoreIOError> {
        match read(self.id, buf) {
            Ok(n) => Ok(n),
            // TODO
            Err(_e) => Err(CoreIOError::InvalidInput),
        }
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> Result<usize, CoreIOError> {
        if buf.is_empty() {
            return Ok(0);
        }
        match write(self.id, buf) {
            Ok(n) => Ok(n),
            // TODO
            Err(_e) => Err(CoreIOError::InvalidInput),
        }
    }

    fn flush(&mut self) -> Result<(), CoreIOError> {
        Ok(())
    }
}
