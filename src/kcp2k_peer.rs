use crate::common::get_current_timestamp_millis;
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::{Kcp2KConfig, METADATA_SIZE_RELIABLE};
use crate::kcp2k_state::Kcp2KState;
use bytes::BufMut;
use kcp::Kcp;
use socket2::{SockAddr, Socket};
use std::io;
use std::io::Write;
use std::sync::Arc;

pub struct Kcp2KPeer {
    pub cookie: Arc<Vec<u8>>, // cookie
    pub state: Kcp2KState,  // 状态
    pub kcp: Kcp<UdpOutput>, // kcp
    pub current_time: u128,
    pub timeout_duration: u128, // 超时时间
    pub last_recv_time: u128, // 最后接收时间
    pub last_send_ping_time: u128, // 最后发送 ping 的时间
}

impl Kcp2KPeer {
    pub fn new(config: Arc<Kcp2KConfig>, cookie: Arc<Vec<u8>>, socket: Arc<Socket>, client_sock_addr: Arc<SockAddr>) -> Self {
        // set up kcp over reliable channel (that's what kcp is for)
        let udp_output = UdpOutput::new(Arc::clone(&cookie), Arc::clone(&socket), Arc::clone(&client_sock_addr));
        // kcp
        let mut kcp = Kcp::new(0, udp_output);
        // set nodelay.
        // note that kcp uses 'nocwnd' internally so we negate the parameter
        kcp.set_nodelay(if config.no_delay { true } else { false }, config.interval as i32, config.fast_resend, !config.congestion_window);
        kcp.set_wndsize(config.send_window_size, config.receive_window_size);

        // IMPORTANT: high level needs to add 1 channel byte to each raw
        // message. so while Kcp.MTU_DEF is perfect, we actually need to
        // tell kcp to use MTU-1 so we can still put the header into the
        // message afterwards.
        kcp.set_mtu(config.mtu - METADATA_SIZE_RELIABLE).expect("TODO: panic message");

        // set maximum retransmits (aka dead_link)
        kcp.set_maximum_resend_times(config.max_retransmits);

        Self {
            kcp,
            cookie,
            state: Kcp2KState::Connected,
            timeout_duration: config.timeout,
            last_recv_time: get_current_timestamp_millis(),
            last_send_ping_time: get_current_timestamp_millis(),
            current_time: get_current_timestamp_millis(),
        }
    }
}

#[derive(Debug)]
pub struct UdpOutput {
    cookie: Arc<Vec<u8>>, // cookie
    socket: Arc<Socket>, // socket
    client_sock_addr: Arc<SockAddr>, // client_sock_addr
}

impl UdpOutput {
    // 创建一个新的 Writer，用于将数据包写入 UdpSocket
    pub fn new(cookie: Arc<Vec<u8>>, socket: Arc<Socket>, client_sock_addr: Arc<SockAddr>) -> UdpOutput {
        UdpOutput {
            cookie,
            socket,
            client_sock_addr,
        }
    }
}

impl Write for UdpOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {

        // 创建一个缓冲区，用于存储消息内容
        let mut buffer = vec![];

        // 写入通道头部
        buffer.put_u8(Kcp2KChannel::Reliable.to_u8());

        // 写入握手 cookie 以防止 UDP 欺骗
        buffer.put_slice(&self.cookie);

        // 写入 data
        buffer.put_slice(buf);

        // 发送数据
        match self.socket.send_to(&buffer, &self.client_sock_addr) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Ok(0),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

