use crate::{
    error::IpcError,
    io::{BufRead, Read},
};
use alloc::vec;
use alloc::vec::Vec;
use core::{cmp, fmt};

const DEFAULT_BUF_SIZE: usize = 1024;

// a simple implementation of BufReader
pub struct BufReader<R: ?Sized> {
    buf: Vec<u8>,
    pos: usize,
    filled: usize,
    inner: R,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> BufReader<R> {
        BufReader::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> BufReader<R> {
        let buf = vec![0; capacity];
        BufReader {
            inner,
            buf,
            pos: 0,
            filled: 0,
        }
    }
}

impl<R: ?Sized> BufReader<R> {
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn into_inner(self) -> R
    where
        R: Sized,
    {
        self.inner
    }

    pub fn discard_buffer(&mut self) {
        self.pos = 0;
        self.filled = 0;
    }
}

impl<R: ?Sized + Read> Read for BufReader<R> {
    type Error = IpcError;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IpcError> {
        if self.pos >= self.filled && buf.len() >= self.capacity() {
            self.discard_buffer();
            return self.inner.read(buf).map_err(|_| IpcError::BufReaderError);
        }

        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}

impl<R: ?Sized + Read> BufRead for BufReader<R> {
    type Error = IpcError;
    fn fill_buf(&mut self) -> Result<&[u8], IpcError> {
        if self.pos >= self.filled {
            assert_eq!(self.pos, self.filled);
            self.filled = self
                .inner
                .read(&mut self.buf)
                .map_err(|_| IpcError::BufReaderError)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.filled])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.filled);
    }
}

impl<R> fmt::Debug for BufReader<R>
where
    R: ?Sized + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BufReader")
            .field("reader", &&self.inner)
            .field(
                "buffer",
                &format_args!("{}/{}", self.filled - self.pos, self.capacity()),
            )
            .finish()
    }
}
