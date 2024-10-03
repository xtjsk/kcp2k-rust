#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Kcp2KChannel {
    None = 0,
    Reliable = 1,
    Unreliable = 2,
}

impl Kcp2KChannel {
    pub fn from(value: u8) -> Kcp2KChannel {
        match value {
            1 => Kcp2KChannel::Reliable,
            2 => Kcp2KChannel::Unreliable,
            _ => Kcp2KChannel::None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}