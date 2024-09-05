// 引入 Rust 标准库中的序列化相关的 trait
use serde::{Deserialize, Serialize};

pub const PING_INTERVAL: u128 = 1000;
pub const CHANNEL_HEADER_SIZE: usize = 1;
pub const COOKIE_HEADER_SIZE: usize = 4;
pub const METADATA_SIZE_RELIABLE: usize = CHANNEL_HEADER_SIZE + COOKIE_HEADER_SIZE;
pub const METADATA_SIZE_UNRELIABLE: usize = CHANNEL_HEADER_SIZE + COOKIE_HEADER_SIZE;

// 定义 KcpConfig 结构体，用于配置 KCP 服务器
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Kcp2KConfig {
    // 使用 IPv6 和 IPv4 的双模式，不是所有平台都支持
    pub dual_mode: bool,
    // UDP 服务器只使用一个 socket，最大化缓冲区以处理尽可能多的连接
    pub recv_buffer_size: usize,
    pub send_buffer_size: usize,
    // 可配置的 MTU，以便 KCP 可以用于其他抽象，如加密传输、中继等
    pub mtu: usize,
    // NoDelay 设置，推荐用于减少延迟
    pub no_delay: bool,
    // KCP 内部更新间隔，建议低于默认的 100ms，以减少延迟和支持更多网络实体
    pub interval: u64,
    // 快速重传参数，以较高的带宽代价换取更快的重传
    pub fast_resend: i32,
    // 拥塞窗口，可能会显著增加延迟，建议禁用
    pub congestion_window: bool,
    // 可修改的 KCP 窗口大小，以支持更高的负载
    pub send_window_size: u16,
    pub receive_window_size: u16,
    // 超时设置，单位为毫秒
    pub timeout: u128,
    // 最大重传次数，直到连接被认为是断开的
    pub max_retransmits: u32,
    pub is_reliable_ping: bool,
}

impl Default for Kcp2KConfig {
    // 提供默认构造函数
    fn default() -> Self {
        Kcp2KConfig {
            dual_mode: false,
            recv_buffer_size: 1024 * 1024 * 7,
            send_buffer_size: 1024 * 1024 * 7,
            mtu: 1200,                  // 假设这是 KCP 默认的 MTU
            no_delay: true,
            interval: 20,
            fast_resend: 0,
            congestion_window: false,
            send_window_size: 32,       // 假设这是发送窗口的默认大小
            receive_window_size: 128,   // 假设这是接收窗口的默认大小
            timeout: 2000,              // 假设这是默认的超时时间
            max_retransmits: 20,        // 假设这是默认的最大重传次数
            is_reliable_ping: true,  // 假设这是默认的可靠 ping
        }
    }
}
