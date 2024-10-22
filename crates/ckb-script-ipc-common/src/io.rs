//!
//! This is a shortened version of standard library's io module.
//! Find documents from standard library.
//!
use crate::error::Error;
use crate::io_impl::{default_read_exact, ReadExactError};

pub trait Read {
    type Error: Error;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ReadExactError> {
        default_read_exact(self, buf)
    }
}

pub trait BufRead {
    type Error: Error;
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error>;
    fn consume(&mut self, amt: usize);
    fn has_data_left(&mut self) -> Result<bool, Self::Error> {
        self.fill_buf().map(|b| !b.is_empty())
    }
}

pub trait Write {
    type Error: Error;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Self::Error> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => panic!("write() returned Ok(0)"),
                Ok(n) => buf = &buf[n..],
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub trait Seek {
    type Error: Error;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error>;
    fn rewind(&mut self) -> Result<(), Self::Error> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }
    fn stream_position(&mut self) -> Result<u64, Self::Error> {
        self.seek(SeekFrom::Current(0))
    }
}
