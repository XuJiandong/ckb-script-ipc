//!
//! This is a shortened version of standard library's io module.
//! Find documents from standard library.
//!
use crate::{
    error::Error,
    io::{Seek, SeekFrom, Write},
};
use alloc::vec::Vec;
use core::{fmt, ptr};

const DEFAULT_BUF_SIZE: usize = 1024;

pub struct BufWriter<W: ?Sized + Write> {
    buf: Vec<u8>,
    panicked: bool,
    inner: W,
}

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(DEFAULT_BUF_SIZE, inner)
    }
    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter {
            inner,
            buf: Vec::with_capacity(capacity),
            panicked: false,
        }
    }
}

impl<W: ?Sized + Write> BufWriter<W> {
    fn flush_buf(&mut self) -> Result<(), <W as Write>::Error> {
        struct BufGuard<'a> {
            buffer: &'a mut Vec<u8>,
            written: usize,
        }

        impl<'a> BufGuard<'a> {
            fn new(buffer: &'a mut Vec<u8>) -> Self {
                Self { buffer, written: 0 }
            }

            fn remaining(&self) -> &[u8] {
                &self.buffer[self.written..]
            }

            fn consume(&mut self, amt: usize) {
                self.written += amt;
            }

            fn done(&self) -> bool {
                self.written >= self.buffer.len()
            }
        }

        impl Drop for BufGuard<'_> {
            fn drop(&mut self) {
                if self.written > 0 {
                    self.buffer.drain(..self.written);
                }
            }
        }

        let mut guard = BufGuard::new(&mut self.buf);
        while !guard.done() {
            self.panicked = true;
            let r = self.inner.write(guard.remaining());
            self.panicked = false;

            match r {
                Ok(n) => guard.consume(n),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    pub fn get_ref(&self) -> &W {
        &self.inner
    }
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }
    #[inline(never)]
    fn write_cold(&mut self, buf: &[u8]) -> Result<usize, <W as Write>::Error> {
        if buf.len() > self.spare_capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write(buf);
            self.panicked = false;
            r
        } else {
            unsafe {
                self.write_to_buffer_unchecked(buf);
            }

            Ok(buf.len())
        }
    }
    #[inline(never)]
    fn write_all_cold(&mut self, buf: &[u8]) -> Result<(), <W as Write>::Error> {
        if buf.len() > self.spare_capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let r = self.get_mut().write_all(buf);
            self.panicked = false;
            r
        } else {
            unsafe {
                self.write_to_buffer_unchecked(buf);
            }

            Ok(())
        }
    }

    #[inline]
    unsafe fn write_to_buffer_unchecked(&mut self, buf: &[u8]) {
        debug_assert!(buf.len() <= self.spare_capacity());
        let old_len = self.buf.len();
        let buf_len = buf.len();
        let src = buf.as_ptr();
        let dst = self.buf.as_mut_ptr().add(old_len);
        ptr::copy_nonoverlapping(src, dst, buf_len);
        self.buf.set_len(old_len + buf_len);
    }

    #[inline]
    fn spare_capacity(&self) -> usize {
        self.buf.capacity() - self.buf.len()
    }
}

impl<W: ?Sized + Write> Write for BufWriter<W> {
    type Error = W::Error;
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.len() < self.spare_capacity() {
            unsafe {
                self.write_to_buffer_unchecked(buf);
            }

            Ok(buf.len())
        } else {
            self.write_cold(buf)
        }
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        if buf.len() < self.spare_capacity() {
            unsafe {
                self.write_to_buffer_unchecked(buf);
            }

            Ok(())
        } else {
            self.write_all_cold(buf)
        }
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_buf().and_then(|()| self.get_mut().flush())
    }
}

impl<W: ?Sized + Write + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BufWriter")
            .field("writer", &&self.inner)
            .field(
                "buffer",
                &format_args!("{}/{}", self.buf.len(), self.buf.capacity()),
            )
            .finish()
    }
}

impl<E: Error, W: ?Sized + Write<Error = E> + Seek<Error = E>> Seek for BufWriter<W> {
    type Error = E;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.flush_buf()?;
        self.get_mut().seek(pos)
    }
}

impl<W: ?Sized + Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        if !self.panicked {
            let _r = self.flush_buf();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bufwriter_write_to_memory() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        writer.write_all(b"Hello").unwrap();
        writer.write_all(b", World!").unwrap();
        writer.flush().unwrap();

        assert_eq!(writer.get_ref(), b"Hello, World!");
    }

    #[test]
    fn test_bufwriter_capacity() {
        let inner = Vec::new();
        let writer = BufWriter::with_capacity(16, inner);

        assert_eq!(writer.capacity(), 16);
    }

    #[test]
    fn test_bufwriter_small_writes() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        writer.write_all(b"a").unwrap();
        writer.write_all(b"b").unwrap();
        writer.write_all(b"c").unwrap();

        assert_eq!(writer.buffer(), b"abc");
        assert_eq!(writer.get_ref().len(), 0); // Not flushed yet

        writer.flush().unwrap();
        assert_eq!(writer.get_ref(), b"abc");
    }

    #[test]
    fn test_bufwriter_large_write() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        writer.write_all(b"abcdefghijklmnop").unwrap();

        assert!(writer.buffer().is_empty()); // Buffer should be flushed
        assert_eq!(writer.get_ref(), b"abcdefghijklmnop");
    }

    #[test]
    fn test_bufwriter_multiple_flushes() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(4, inner);

        writer.write_all(b"ab").unwrap();
        writer.flush().unwrap();
        writer.write_all(b"cd").unwrap();
        writer.flush().unwrap();

        assert_eq!(writer.get_ref(), b"abcd");
    }

    #[test]
    fn test_write_exact_capacity() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        let bytes_written = writer.write(b"12345678").unwrap();
        assert_eq!(bytes_written, 8);
        assert!(writer.buffer().is_empty());
        assert_eq!(writer.get_ref(), b"12345678");
    }

    #[test]
    fn test_write_over_capacity() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        let bytes_written = writer.write(b"123456789").unwrap();
        assert_eq!(bytes_written, 9);
        assert!(writer.buffer().is_empty());
        assert_eq!(writer.get_ref(), b"123456789");
    }

    #[test]
    fn test_multiple_writes_within_capacity() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        assert_eq!(writer.write(b"123").unwrap(), 3);
        assert_eq!(writer.write(b"456").unwrap(), 3);
        assert_eq!(writer.buffer(), b"123456");
        assert!(writer.get_ref().is_empty());
    }

    #[test]
    fn test_write_zero_bytes() {
        let inner = Vec::new();
        let mut writer = BufWriter::with_capacity(8, inner);

        let bytes_written = writer.write(&[]).unwrap();
        assert_eq!(bytes_written, 0);
        assert!(writer.buffer().is_empty());
        assert!(writer.get_ref().is_empty());
    }
}
