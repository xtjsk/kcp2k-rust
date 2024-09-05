#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Kcp2KHeaderReliable {
    Hello = 1,
    Ping = 2,
    Data = 3,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Kcp2KHeaderUnreliable {
    Data = 4,
    Disconnect = 5,
    Ping = 6,
}

impl Kcp2KHeaderReliable {
    pub fn parse(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Hello),
            2 => Some(Self::Ping),
            3 => Some(Self::Data),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

impl Kcp2KHeaderUnreliable {
    pub fn parse(value: u8) -> Option<Self> {
        match value {
            4 => Some(Self::Data),
            5 => Some(Self::Disconnect),
            6 => Some(Self::Ping),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}