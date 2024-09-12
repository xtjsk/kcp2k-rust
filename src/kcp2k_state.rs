
#[derive(Debug, PartialEq, Clone)]
pub enum Kcp2KPeerState {
    Connected,
    Authenticated,
    Disconnected,
}