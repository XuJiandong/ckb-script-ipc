pub enum Cmd {
    CkbBlake2b = 0,
    Blake2b,
    Sha256,
    Ripemd160,
}

impl From<u8> for Cmd {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::CkbBlake2b,
            1 => Self::Blake2b,
            2 => Self::Sha256,
            3 => Self::Ripemd160,
            _ => {
                panic!("unknow Val");
            }
        }
    }
}

impl From<Cmd> for u8 {
    fn from(value: Cmd) -> Self {
        value as u8
    }
}
