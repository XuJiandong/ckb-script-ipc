#[repr(i8)]
pub enum Error {
    Unknown = 1,
    CkbSysError,
    ServerError,
}
