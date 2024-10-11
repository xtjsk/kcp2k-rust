#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Kcp2KPeerState {
    Connected,
    Authenticated,
    Disconnected,
}
