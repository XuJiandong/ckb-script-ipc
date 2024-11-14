pub enum Cmd {
    Blake2b = 0,
    Sha256,
}

impl From<u8> for Cmd {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Blake2b,
            1 => Self::Sha256,
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
