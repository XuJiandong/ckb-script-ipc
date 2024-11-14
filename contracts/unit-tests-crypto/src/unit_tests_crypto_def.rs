use ckb_std::ckb_types::packed::Byte;

pub enum Cmd {
    Blake2b,
}

impl From<u8> for Cmd {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Blake2b,
            _ => {
                panic!("unknow Val");
            }
        }
    }
}

impl From<Byte> for Cmd {
    fn from(value: Byte) -> Self {
        Cmd::from(u8::from(value))
    }
}
