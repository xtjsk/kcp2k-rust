use crate::error_code::ErrorCode;
use crate::kcp2k_channel::Kcp2KChannel;
use bytes::Bytes;
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
    pub r#type: CallbackType,
    pub conn_id: u64,
    pub data: Bytes,
    pub channel: Kcp2KChannel,
    pub error_code: ErrorCode,
    pub error_message: String,
}
impl Debug for Callback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.r#type {
            CallbackType::OnConnected => {
                write!(f, "OnConnected: id {} ", self.conn_id)
            }
            CallbackType::OnData => {
                write!(
                    f,
                    "OnData: id {} {:?} {:?}",
                    self.conn_id, self.channel, self.data
                )
            }
            CallbackType::OnDisconnected => {
                write!(f, "OnDisconnected: id {}", self.conn_id)
            }
            CallbackType::OnError => {
                write!(
                    f,
                    "OnError: id {} - {:?} {}",
                    self.conn_id, self.error_code, self.error_message
                )
            }
        }
    }
}

impl Default for Callback {
    fn default() -> Self {
        Self {
            r#type: CallbackType::OnError,
            data: Bytes::new(),
            conn_id: 0,
            channel: Kcp2KChannel::None,
            error_code: ErrorCode::None,
            error_message: "None".to_string(),
        }
    }
}
