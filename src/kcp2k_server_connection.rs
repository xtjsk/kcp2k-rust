use crate::common::{get_current_timestamp_millis, Kcp2KMode};
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::{Callback, CallbackType};
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_config::PING_INTERVAL;
use crate::kcp2k_header::{Kcp2KHeaderReliable, Kcp2KHeaderUnreliable};
use crate::kcp2k_peer::Kcp2KPeer;
use crate::kcp2k_state::Kcp2KState;
use bytes::BufMut;
use socket2::{SockAddr, Socket};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// KcpServerConnection
pub struct Kcp2KServerConnection {
    socket: Arc<Socket>,
    removed_connections: Arc<Mutex<Vec<u64>>>, // removed_connections
    kcp2k_mode: Arc<Kcp2KMode>,
    connection_id: u64,
    client_sock_addr: Arc<SockAddr>,
    callback_tx: mpsc::UnboundedSender<Callback>,
    kcp_peer: Kcp2KPeer,
    is_reliable_ping: bool,
}

impl Kcp2KServerConnection {
    pub fn new(config: Arc<Kcp2KConfig>, cookie: Arc<Vec<u8>>, socket: Arc<Socket>, connection_id: u64, client_sock_addr: Arc<SockAddr>, callback_tx: mpsc::UnboundedSender<Callback>, removed_connections: Arc<Mutex<Vec<u64>>>, kcp2k_mode: Arc<Kcp2KMode>) -> Self {
        let mut kcp_server_connection = Kcp2KServerConnection {
            socket: Arc::clone(&socket),
            removed_connections,
            kcp2k_mode,
            connection_id,
            client_sock_addr: Arc::clone(&client_sock_addr),
            callback_tx: callback_tx.clone(),
            kcp_peer: Kcp2KPeer::new(Arc::clone(&config), Arc::clone(&cookie), Arc::clone(&socket), Arc::clone(&client_sock_addr)),
            is_reliable_ping: config.is_reliable_ping,
        };
        if kcp_server_connection.kcp2k_mode == Arc::from(Kcp2KMode::Client) {
            let _ = kcp_server_connection.send_hello();
        } else if kcp_server_connection.kcp2k_mode == Arc::from(Kcp2KMode::Server) {
            println!("[KCP2K] {:?} created connection with cookie={:?}", kcp_server_connection.kcp2k_mode, cookie);
        }
        kcp_server_connection
    }
    pub fn set_kcp_peer(&mut self, kcp_peer: Kcp2KPeer) {
        self.kcp_peer = kcp_peer;
    }
    pub fn get_connection_id(&self) -> u64 {
        self.connection_id
    }
    pub fn set_connection_id(&mut self, connection_id: u64) {
        self.connection_id = connection_id;
    }
    fn on_connected(&mut self) -> Result<(), ErrorCode> {
        match self.callback_tx.send(Callback {
            callback_type: CallbackType::OnConnected,
            connection_id: self.connection_id,
            ..Default::default()
        }) {
            Ok(_) => Ok(()),
            Err(_) => Err(ErrorCode::SendError)
        }
    }
    fn on_authenticated(&mut self) -> Result<(), ErrorCode> {
        self.kcp_peer.state = Kcp2KState::Authenticated;
        let _ = self.send_hello();
        self.on_connected()
    }
    fn on_data(&mut self, data: &[u8], kcp2k_channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        match self.callback_tx.send(Callback {
            callback_type: CallbackType::OnData,
            data: data.to_vec(),
            channel: kcp2k_channel,
            connection_id: self.connection_id,
            ..Default::default()
        }) {
            Ok(_) => Ok(()),
            Err(_) => Err(ErrorCode::SendError)
        }
    }
    fn on_disconnected(&mut self) -> Result<(), ErrorCode> {
        // 如果连接已经断开，则不执行任何操作
        if self.kcp_peer.state == Kcp2KState::Disconnected {
            return Ok(());
        }
        // 发送断开消息
        self.send_disconnect();
        // 设置状态为断开
        self.kcp_peer.state = Kcp2KState::Disconnected;
        // 添加到移除列表
        self.removed_connections.lock().unwrap().push(self.connection_id);
        // 发送回调
        if let Ok(_) = self.callback_tx.send(Callback {
            callback_type: CallbackType::OnDisconnected,
            connection_id: self.connection_id,
            ..Default::default()
        }) {
            Ok(())
        } else {
            Err(ErrorCode::SendError)
        }
    }
    fn on_error(&mut self, error_code: ErrorCode, error_message: String) -> Result<(), ErrorCode> {
        match self.callback_tx.send(Callback {
            callback_type: CallbackType::OnError,
            connection_id: self.connection_id,
            error_code,
            error_message,
            ..Default::default()
        }) {
            Ok(_) => Ok(()),
            Err(_) => Err(ErrorCode::SendError)
        }
    }
    fn on_raw_send(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        match self.socket.send_to(&data, &self.client_sock_addr) {
            Ok(_) => Ok(()),
            Err(_) => Err(ErrorCode::SendError)
        }
    }
    pub fn on_raw_input(&mut self, segment: Vec<u8>) -> Result<(), ErrorCode> {
        if segment.len() <= 5 {
            return Err(ErrorCode::InvalidReceive);
        }

        // 类型
        let channel = segment[0];

        // cookie
        let cookie = &segment[1..5];

        // 如果连接已经通过验证，但是收到了带有不同 cookie 的消息，那么这可能是由于客户端的 Hello 消息被多次传输，或者攻击者尝试进行 UDP 欺骗。
        if self.kcp_peer.state == Kcp2KState::Authenticated {
            if cookie != self.kcp_peer.cookie.to_vec() {
                self.on_error(ErrorCode::InvalidReceive, format!("{}: Dropped message with invalid cookie: {:?} from {:?} expected: {:?} state: {:?}. This can happen if the client's Hello message was transmitted multiple times, or if an attacker attempted UDP spoofing.", std::any::type_name::<Self>(), cookie, self.client_sock_addr.clone(), self.kcp_peer.cookie.to_vec(), self.kcp_peer.state))?;
                return Err(ErrorCode::InvalidReceive);
            }
        }

        // 消息
        let message = &segment[5..];

        self.kcp_peer.last_recv_time = get_current_timestamp_millis();

        // 根据通道类型处理消息
        match Kcp2KChannel::from(channel) {
            Kcp2KChannel::Reliable => self.on_raw_input_reliable(message),
            Kcp2KChannel::Unreliable => self.on_raw_input_unreliable(message),
            _ => Err(ErrorCode::Unexpected)
        }
    }
    fn receive_next_reliable(&mut self) -> Option<(Kcp2KHeaderReliable, Vec<u8>)> {
        // 用于存储接收到的数据
        let mut buffer = vec![];
        // 初始化 buffer 大小
        match self.kcp_peer.kcp.peeksize() {
            Ok(size) => {
                buffer.resize(size, 0);
            }
            Err(_) => {
                return None;
            }
        }
        // 从 KCP 接收数据
        match self.kcp_peer.kcp.recv(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    let _ = self.on_error(ErrorCode::InvalidReceive, format!("{}: Receive failed with error={}. closing connection.", std::any::type_name::<Self>(), size));
                    let _ = self.on_disconnected();
                    return None;
                }
                // 解析头部
                let header_byte = buffer[0];
                match Kcp2KHeaderReliable::parse(header_byte) {
                    Some(header) => {
                        // 从 buffer 中提取消息
                        Some((header, buffer[1..size].to_vec()))
                    }
                    None => {
                        let _ = self.on_error(ErrorCode::InvalidReceive, format!("[KCP-2K] {}: Receive failed to parse header: {} is not defined in {}.", std::any::type_name::<Self>(), header_byte, std::any::type_name::<Kcp2KHeaderReliable>()));
                        let _ = self.on_disconnected();
                        None
                    }
                }
            }
            Err(error) => {
                let _ = self.on_error(ErrorCode::InvalidReceive, format!("[KCP-2K] connection - {}: Receive failed with error={}. closing connection.", std::any::type_name::<Self>(), error));
                let _ = self.on_disconnected();
                None
            }
        }
    }
    fn on_raw_input_reliable(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        if let Err(e) = self.kcp_peer.kcp.input(&data) {
            self.on_error(ErrorCode::InvalidReceive,
                          format!("[KCP2K] {}: Input failed with error={:?} for buffer with length={}",
                                  std::any::type_name::<Self>(),
                                  e,
                                  data.len() - 1))
        } else {
            Ok(())
        }
    }
    fn on_raw_input_unreliable(&mut self, data: &[u8]) -> Result<(), ErrorCode> {
        // 至少需要一个字节用于 header
        if data.len() < 1 {
            return Err(ErrorCode::InvalidReceive);
        }
        // 安全地提取标头。攻击者可能会发送超出枚举范围的值。
        let header = data[0];

        // 判断 header 类型
        let header = match Kcp2KHeaderUnreliable::parse(header) {
            Some(header) => header,
            None => {
                let _ = self.on_disconnected();
                self.on_error(
                    ErrorCode::InvalidReceive,
                    format!("{}: Receive failed to parse header: {} is not defined in {}.",
                            std::any::type_name::<Self>(),
                            header,
                            std::any::type_name::<Kcp2KHeaderUnreliable>()
                    ),
                )?;
                return Err(ErrorCode::InvalidReceive);
            }
        };

        // 提取数据
        let data = &data[1..];

        // 根据头部类型处理消息
        match header {
            Kcp2KHeaderUnreliable::Data => {
                match self.kcp_peer.state {
                    Kcp2KState::Authenticated => {
                        self.on_data(&data, Kcp2KChannel::Unreliable)
                    }
                    _ => {
                        self.on_error(ErrorCode::InvalidReceive, format!("{}: Received Data message while not Authenticated. Disconnecting the connection.", std::any::type_name::<Self>()))
                    }
                }
            }
            Kcp2KHeaderUnreliable::Disconnect => {
                self.on_disconnected()
            }
            Kcp2KHeaderUnreliable::Ping => Ok(())
        }
    }
    fn send_reliable(&mut self, kcp2k_header_reliable: Kcp2KHeaderReliable, data: Vec<u8>) -> Result<(), ErrorCode> {
        // 创建一个缓冲区，用于存储消息内容
        let mut buffer = vec![];

        // 写入通道头部
        buffer.put_u8(kcp2k_header_reliable.to_u8());

        // 写入数据
        if !data.is_empty() {
            buffer.put_slice(&data);
        }


        // 通过 KCP 发送处理
        match self.kcp_peer.kcp.send(&buffer) {
            Ok(_) => {
                let _ = self.kcp_peer.kcp.flush();
                Ok(())
            }
            Err(e) => {
                self.on_error(ErrorCode::InvalidSend, format!("{}: 发送失败，错误码={}，内容长度={}", "send_reliable", e, data.len()))
            }
        }
    }
    fn send_unreliable(&mut self, kcp2k_header_unreliable: Kcp2KHeaderUnreliable, data: Vec<u8>) -> Result<(), ErrorCode> {
        // 创建一个缓冲区，用于存储消息内容
        let mut buffer = vec![];

        // 写入通道头部
        buffer.put_u8(Kcp2KChannel::Unreliable.to_u8());

        // 写入握手 cookie 以防止 UDP 欺骗
        buffer.put_slice(&self.kcp_peer.cookie);

        // 写入 kcp 头部
        buffer.put_u8(kcp2k_header_unreliable.to_u8());

        // 写入数据
        if !data.is_empty() {
            buffer.put_slice(&data);
        }
        // raw send
        self.on_raw_send(&buffer)
    }
    pub fn tick_incoming(&mut self) {
        // 更新时间
        self.kcp_peer.current_time = get_current_timestamp_millis();
        // 根据状态处理不同的逻辑
        match self.kcp_peer.state {
            Kcp2KState::Connected => self.tick_incoming_connected(),
            Kcp2KState::Authenticated => self.tick_incoming_authenticated(),
            Kcp2KState::Disconnected => {}
        }
    }
    pub fn tick_outgoing(&mut self) {
        match self.kcp_peer.state {
            Kcp2KState::Connected | Kcp2KState::Authenticated => {
                let _ = self.kcp_peer.kcp.update((self.kcp_peer.current_time / 1000) as u32);
            }
            Kcp2KState::Disconnected => {}
        }
    }
    // 处理连接
    fn tick_incoming_connected(&mut self) {
        self.handle_ping();
        self.handle_timeout();
        self.handle_dead_link();

        while let Some((header, _)) = self.receive_next_reliable() {
            match header {
                Kcp2KHeaderReliable::Hello => {
                    let _ = self.on_authenticated();
                }
                Kcp2KHeaderReliable::Data => {
                    let _ = self.on_error(ErrorCode::InvalidReceive, "Received invalid header while Connected. Disconnecting the connection.".to_string());
                    let _ = self.on_disconnected();
                }
                Kcp2KHeaderReliable::Ping => {}
            }
        }
    }
    // 处理认证
    fn tick_incoming_authenticated(&mut self) {
        self.handle_ping();
        self.handle_timeout();
        self.handle_dead_link();

        while let Some((header, data)) = self.receive_next_reliable() {
            match header {
                Kcp2KHeaderReliable::Hello => {
                    let _ = self.on_error(ErrorCode::InvalidReceive, "Received invalid header while Authenticated. Disconnecting the connection.".to_string());
                    let _ = self.on_disconnected();
                }
                Kcp2KHeaderReliable::Data => {
                    if data.is_empty() {
                        let _ = self.on_error(ErrorCode::InvalidReceive, "Received empty Data message while Authenticated. Disconnecting the connection.".to_string());
                        let _ = self.on_disconnected();
                    } else {
                        let _ = self.on_data(&data, Kcp2KChannel::Reliable);
                    }
                }
                Kcp2KHeaderReliable::Ping => {}
            }
        }
    }
    // 发送 hello
    fn send_hello(&mut self) -> Result<(), ErrorCode> {
        println!("[KCP2K] {:?} send handshake to server with cookie={:?}", self.kcp2k_mode, self.kcp_peer.cookie);
        self.send_reliable(Kcp2KHeaderReliable::Hello, Default::default())
    }
    // 发送 ping
    fn send_ping(&mut self) -> Result<(), ErrorCode> {
        self.kcp_peer.last_send_ping_time = get_current_timestamp_millis();
        if self.is_reliable_ping {
            self.send_reliable(Kcp2KHeaderReliable::Ping, Default::default())
        } else {
            self.send_unreliable(Kcp2KHeaderUnreliable::Ping, Default::default())
        }
    }
    // 发送数据
    pub fn send_data(&mut self, data: Vec<u8>, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        // 如果数据为空，则返回错误
        if data.is_empty() {
            self.on_error(ErrorCode::InvalidSend, "send_data: tried sending empty message. This should never happen. Disconnecting.".to_string())?;
            let _ = self.on_disconnected();
            return Err(ErrorCode::SendError);
        }
        // 根据通道类型发送数据
        match channel {
            Kcp2KChannel::Reliable => {
                self.send_reliable(Kcp2KHeaderReliable::Data, data)
            }
            Kcp2KChannel::Unreliable => {
                self.send_unreliable(Kcp2KHeaderUnreliable::Data, data)
            }
            _ => {
                self.on_error(ErrorCode::InvalidSend, format!("send_data: tried sending message with invalid channel: {:?}. Disconnecting.", channel))?;
                let _ = self.on_disconnected();
                Err(ErrorCode::SendError)
            }
        }
    }
    // 发送断开连接
    fn send_disconnect(&mut self) {
        for _ in 0..5 {
            let _ = self.send_unreliable(Kcp2KHeaderUnreliable::Disconnect, Default::default());
        }
    }
    // 处理 ping
    fn handle_ping(&mut self) {
        if self.kcp_peer.current_time > self.kcp_peer.last_send_ping_time + PING_INTERVAL {
            let _ = self.send_ping();
        }
    }
    // 处理超时
    fn handle_timeout(&mut self) {
        if self.kcp_peer.current_time > self.kcp_peer.last_recv_time + self.kcp_peer.timeout_duration {
            let _ = self.on_error(ErrorCode::Timeout, "to disconnected.".to_string());
            let _ = self.on_disconnected();
        }
    }
    // 处理 dead_link
    fn handle_dead_link(&mut self) {
        if self.kcp_peer.kcp.is_dead_link() {
            let _ = self.on_error(ErrorCode::Timeout, "dead_link detected: a message was retransmitted times without ack. Disconnecting.".to_string());
            let _ = self.on_disconnected();
        }
    }
}
