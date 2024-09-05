
#[derive(Debug, PartialEq, Clone)]
pub enum Kcp2KState {
    Connected,
    Authenticated,
    Disconnected,
}