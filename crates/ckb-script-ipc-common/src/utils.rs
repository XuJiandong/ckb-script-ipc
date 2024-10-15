use crate::error::IpcError;
use crate::io::Read;

pub(crate) fn default_read_exact<R: Read + ?Sized>(
    this: &mut R,
    mut buf: &mut [u8],
) -> Result<(), IpcError> {
    while !buf.is_empty() {
        match this.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                buf = &mut buf[n..];
            }
            Err(_) => return Err(IpcError::ReadExactError),
        }
    }
    if !buf.is_empty() {
        Err(IpcError::ReadExactError)
    } else {
        Ok(())
    }
}

/// It is identical to `std::io::Read::read_exact`.
/// Move this to here to relax the dependency of Error trait in `Read`.
/// It returns `IpcError` only.
pub fn read_exact<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<(), IpcError> {
    default_read_exact(reader, buf)
}
