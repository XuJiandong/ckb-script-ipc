pub enum Cmd {
    CkbBlake2b = 0,
    Blake2b,
    Sha256,
    Ripemd160,
    Secp256k1Recover,
    Secp256k1Verify,
    Ed25519Verfiy,
}

impl From<u8> for Cmd {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::CkbBlake2b,
            1 => Self::Blake2b,
            2 => Self::Sha256,
            3 => Self::Ripemd160,
            4 => Self::Secp256k1Recover,
            5 => Self::Secp256k1Verify,
            6 => Self::Ed25519Verfiy,
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
