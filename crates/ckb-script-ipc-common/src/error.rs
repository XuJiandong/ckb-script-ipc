use ckb_std::error::SysError;
use core::fmt::{self, Debug, Display};
use enumn::N;

// use core::error::Error when Rust 1.81 is used.
pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum IpcError {
    CkbSysError(SysError),
    UnexpectedEof,
    IncompleteVlqSeq,
    DecodeVlqOverflow,
    ReadVlqError,
    SerializeError,
    DeserializeError,
    SliceWriteError,
    ReadUntilError,
    ReadExactError,
    BufReaderError,
    ProtocolError(ProtocolErrorCode),
}

impl Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for IpcError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Protocol error code used in wire protocol.
/// Its range from 1 to 2^64 - 1.
/// 1~20 are with same values used in syscall error.
#[derive(Debug, Clone, N)]
#[repr(u64)]
pub enum ProtocolErrorCode {
    Ok = 0,
    /// Index out of bound
    IndexOutOfBound = 1,
    /// Field is missing for the target
    ItemMissing = 2,
    /// Buffer length is not enough, error contains actual data length
    LengthNotEnough = 3,
    /// Data encoding error(molecule)
    InvalidData = 4,
    /// Failed to wait.
    WaitFailure = 5,
    /// Invalid file descriptor.
    InvalidFd = 6,
    /// Reading from or writing to file descriptor failed due to other end closed.
    OtherEndClosed = 7,
    /// Max vms has been spawned.
    MaxVmsSpawned = 8,
    /// Max fds has been spawned.
    MaxFdsCreated = 9,

    /// Unknown error code
    UnknownError = 20,
    /// Unknown error from SysError in ckb-std
    UnknownSysError = 21,
    /// Unexpected EOF
    UnexpectedEof = 22,
    /// VQL error: incomplete VLQ sequence
    IncompleteVlqSeq = 23,
    /// VLQ error: decoding overflow
    DecodeVlqOverflow = 24,
    /// VLQ error: reading error
    ReadVlqError = 25,
    /// Serialize error
    SerializeError = 26,
    /// Deserialize error
    DeserializeError = 27,
    /// general IO error
    GeneralIoError = 28,

    // increase when appending new error codes
    EndOfError = 29,
}

impl From<IpcError> for ProtocolErrorCode {
    fn from(err: IpcError) -> Self {
        match err {
            IpcError::CkbSysError(err) => match err {
                SysError::IndexOutOfBound => ProtocolErrorCode::IndexOutOfBound,
                SysError::ItemMissing => ProtocolErrorCode::ItemMissing,
                SysError::LengthNotEnough(_) => ProtocolErrorCode::LengthNotEnough,
                SysError::Encoding => ProtocolErrorCode::InvalidData,
                SysError::WaitFailure => ProtocolErrorCode::WaitFailure,
                SysError::InvalidFd => ProtocolErrorCode::InvalidFd,
                SysError::OtherEndClosed => ProtocolErrorCode::OtherEndClosed,
                SysError::MaxVmsSpawned => ProtocolErrorCode::MaxVmsSpawned,
                SysError::MaxFdsCreated => ProtocolErrorCode::MaxFdsCreated,
                _ => ProtocolErrorCode::UnknownSysError,
            },
            IpcError::UnexpectedEof => ProtocolErrorCode::UnexpectedEof,
            IpcError::IncompleteVlqSeq => ProtocolErrorCode::IncompleteVlqSeq,
            IpcError::DecodeVlqOverflow => ProtocolErrorCode::DecodeVlqOverflow,
            IpcError::ReadVlqError => ProtocolErrorCode::ReadVlqError,
            IpcError::SerializeError => ProtocolErrorCode::SerializeError,
            IpcError::DeserializeError => ProtocolErrorCode::DeserializeError,
            IpcError::SliceWriteError
            | IpcError::BufReaderError
            | IpcError::ReadUntilError
            | IpcError::ReadExactError => ProtocolErrorCode::GeneralIoError,
            IpcError::ProtocolError(e) => e,
        }
    }
}

impl From<u64> for ProtocolErrorCode {
    fn from(e: u64) -> Self {
        Self::n(e).unwrap()
    }
}
