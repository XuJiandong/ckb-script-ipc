//!
//! This is a shortened version of standard library's io module.
//! Find documents from standard library.
//!
use crate::error::Error;
pub trait Read {
    type Error: Error;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
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

impl<T: ?Sized + Read> Read for &mut T {
    type Error = T::Error;
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, buf)
    }
}

impl<T: ?Sized + BufRead> BufRead for &mut T {
    type Error = T::Error;
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        T::fill_buf(self)
    }

    fn consume(&mut self, amt: usize) {
        T::consume(self, amt);
    }
}

impl<T: ?Sized + Write> Write for &mut T {
    type Error = T::Error;
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        T::write(self, buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        T::flush(self)
    }
}

impl<T: ?Sized + Seek> Seek for &mut T {
    type Error = T::Error;
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        T::seek(self, pos)
    }
}
