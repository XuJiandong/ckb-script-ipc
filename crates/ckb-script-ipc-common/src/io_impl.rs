//!
//! This is a shortened version of standard library's io module.
//! Find documents from standard library.
//!
extern crate alloc;
use crate::error::IpcError;
use crate::io::{BufRead, Read, Seek, SeekFrom, Write};
use alloc::boxed::Box;
use alloc::vec::Vec;

impl<T: ?Sized + Read> Read for Box<T> {
    type Error = T::Error;
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, buf)
    }
}

impl<T: ?Sized + BufRead> BufRead for Box<T> {
    type Error = T::Error;
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        T::fill_buf(self)
    }

    fn consume(&mut self, amt: usize) {
        T::consume(self, amt);
    }
}

impl<T: ?Sized + Write> Write for Box<T> {
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

impl<T: ?Sized + Seek> Seek for Box<T> {
    type Error = T::Error;
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        T::seek(self, pos)
    }
}

impl Write for &mut [u8] {
    type Error = IpcError;
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let amt = core::cmp::min(buf.len(), self.len());
        if !buf.is_empty() && amt == 0 {
            return Err(IpcError::SliceWriteError);
        }
        let (a, b) = core::mem::take(self).split_at_mut(amt);
        a.copy_from_slice(&buf[..amt]);
        *self = b;
        Ok(amt)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Read for &[u8] {
    type Error = IpcError;
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let amt = core::cmp::min(buf.len(), self.len());
        let (a, b) = self.split_at(amt);
        if amt == 1 {
            buf[0] = a[0];
        } else {
            buf[..amt].copy_from_slice(a);
        }

        *self = b;
        Ok(amt)
    }
}

impl BufRead for &[u8] {
    type Error = IpcError;
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        Ok(*self)
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        *self = &self[amt..];
    }
}

impl Write for Vec<u8> {
    type Error = IpcError;
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::io::Read;
    #[test]
    fn test_io_impl() {
        let array = [1u8, 2, 3, 4];
        let mut buf = &array[..];
        let buf2 = &mut [0u8; 2];
        buf.read(buf2).unwrap();
        assert_eq!(buf.len(), 2);
    }
}
