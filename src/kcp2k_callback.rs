use crate::error_code::ErrorCode;
use crate::kcp2k_channel::Kcp2KChannel;
use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub enum CallbackType {
    OnConnected,
    OnData,
    OnDisconnected,
    OnError,
}

// Callback: 服务器回调
pub struct Callback {
    pub callback_type: CallbackType,
    pub connection_id: u64,
    pub data: Vec<u8>,
    pub channel: Kcp2KChannel,
    pub error_code: ErrorCode,
    pub error_message: String,
}
impl Debug for Callback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.callback_type {
            CallbackType::OnConnected => {
                write!(f, "OnConnected: id {} ", self.connection_id)
            }
            CallbackType::OnData => {
                write!(f, "OnData: id {} {:?} {:?}", self.connection_id, self.channel, self.data)
            }
            CallbackType::OnDisconnected => {
                write!(f, "OnDisconnected: id {}", self.connection_id)
            }
            CallbackType::OnError => {
                write!(f, "OnError: id {} - {:?} {}", self.connection_id, self.error_code, self.error_message)
            }
        }
    }
}

impl Default for Callback {
    fn default() -> Self {
        Self {
            callback_type: CallbackType::OnError,
            data: vec![],
            connection_id: 0,
            channel: Kcp2KChannel::None,
            error_code: ErrorCode::None,
            error_message: "default".to_string(),
        }
    }
}