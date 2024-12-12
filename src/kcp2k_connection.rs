use crate::common::Kcp2KMode;
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::{Callback, CallbackType};
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_header::{Kcp2KHeaderReliable, Kcp2KHeaderUnreliable};
use crate::kcp2k_peer::Kcp2KPeer;
use crate::kcp2k_state::Kcp2KPeerState;
use bytes::{BufMut, Bytes, BytesMut};
use socket2::{SockAddr, Socket};
use std::sync::Arc;
use std::time::Duration;
use tklog::error;

// KcpServerConnection
#[derive(Debug)]
pub struct Kcp2KConnection {
    socket: Arc<Socket>,
    connection_id: u64,
    client_sock_addr: Arc<SockAddr>,
    callback: fn(Callback),
    remove_connection_tx: Arc<crossbeam_channel::Sender<u64>>,
    kcp_peer: Kcp2KPeer,
    is_reliable_ping: bool,
}

impl Kcp2KConnection {
    pub fn new(
        config: Arc<Kcp2KConfig>,
        cookie: Arc<Bytes>,
        socket: Arc<Socket>,
        connection_id: u64,
        client_sock_addr: Arc<SockAddr>,
        kcp2k_mode: Arc<Kcp2KMode>,
        callback: fn(Callback),
        remove_connection_tx: Arc<crossbeam_channel::Sender<u64>>,
    ) -> Self {
        let kcp_server_connection = Kcp2KConnection {
            socket: Arc::clone(&socket),
            connection_id,
            client_sock_addr: Arc::clone(&client_sock_addr),
            callback,
            remove_connection_tx,
            kcp_peer: Kcp2KPeer::new(
                Arc::clone(&kcp2k_mode),
                Arc::clone(&config),
                Arc::clone(&cookie),
                Arc::clone(&socket),
                Arc::clone(&client_sock_addr),
            ),
            is_reliable_ping: config.is_reliable_ping,
        };
        if kcp2k_mode == Arc::from(Kcp2KMode::Client) {
            let _ = kcp_server_connection.send_hello();
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
    fn on_connected(&self) {
        (self.callback)(Callback {
            callback_type: CallbackType::OnConnected,
            connection_id: self.connection_id,
            ..Default::default()
        });
    }
    fn on_authenticated(&self) {
        self.send_hello();
        match self.kcp_peer.state.try_write() {
            Ok(mut state) => {
                *state = Kcp2KPeerState::Authenticated;
                self.on_connected()
            }
            Err(err) => {
                error!(format!(
                    "{}: Failed to set state to Authenticated: {}",
                    std::any::type_name::<Self>(),
                    err
                ));
            }
        };
    }
    fn on_data(&self, data: Bytes, kcp2k_channel: Kcp2KChannel) {
        (self.callback)(Callback {
            callback_type: CallbackType::OnData,
            data,
            channel: kcp2k_channel,
            connection_id: self.connection_id,
            ..Default::default()
        });
    }
    fn on_disconnected(&self) {
        // 如果连接已经断开，则不执行任何操作
        match self.kcp_peer.state.try_read() {
            Ok(state) => {
                if *state == Kcp2KPeerState::Disconnected {
                    return;
                }
            }
            Err(err) => {
                error!(format!(
                    "{}: Failed to read state: {}",
                    std::any::type_name::<Self>(),
                    err
                ));
            }
        }
        // 发送断开消息
        self.send_disconnect();
        // 设置状态为断开
        match self.kcp_peer.state.try_write() {
            Ok(mut state) => {
                *state = Kcp2KPeerState::Disconnected;
            }
            Err(err) => {
                error!(format!(
                    "{}: Failed to set state to Disconnected: {}",
                    std::any::type_name::<Self>(),
                    err
                ));
            }
        }
        // 回调
        (self.callback)(Callback {
            callback_type: CallbackType::OnDisconnected,
            connection_id: self.connection_id,
            ..Default::default()
        });
    }
    fn on_error(&self, error_code: ErrorCode, error_message: String) {
        (self.callback)(Callback {
            callback_type: CallbackType::OnError,
            connection_id: self.connection_id,
            error_code,
            error_message,
            ..Default::default()
        });
    }
    fn raw_send(&self, data: &[u8]) -> Result<(), ErrorCode> {
        match self.socket.send_to(&data, &self.client_sock_addr) {
            Ok(_) => Ok(()),
            Err(_) => Err(ErrorCode::SendError),
        }
    }
    pub fn raw_input(&mut self, segment: Bytes) -> Result<(), ErrorCode> {
        if segment.len() <= 5 {
            self.on_error(
                ErrorCode::InvalidReceive,
                format!(
                    "{}: Received invalid message with length={}. Disconnecting the connection.",
                    std::any::type_name::<Self>(),
                    segment.len()
                ),
            );
            return Err(ErrorCode::InvalidReceive);
        }

        // cookie
        let cookie = Arc::from(Bytes::copy_from_slice(&segment[1..5]));

        // 如果连接已经通过验证，但是收到了带有不同 cookie 的消息，那么这可能是由于客户端的 Hello 消息被多次传输，或者攻击者尝试进行 UDP 欺骗。
        match self.kcp_peer.state.try_read() {
            Ok(state) => {
                if *state == Kcp2KPeerState::Authenticated {
                    if cookie != self.kcp_peer.cookie {
                        self.on_error(
                            ErrorCode::InvalidReceive,
                            format!(
                                "{}: Dropped message with invalid cookie: {:?} from {:?} expected: {:?} state: {:?}. This can happen if the client's Hello message was transmitted multiple times, or if an attacker attempted UDP spoofing.",
                                std::any::type_name::<Self>(),
                                cookie.to_vec(),
                                self.client_sock_addr.clone(),
                                self.kcp_peer.cookie.to_vec(),
                                self.kcp_peer.state
                            ),
                        );
                        self.send_disconnect();
                        return Err(ErrorCode::InvalidReceive);
                    }
                }
            }
            Err(err) => {
                error!(format!(
                    "{}: Failed to read state: {}",
                    std::any::type_name::<Self>(),
                    err
                ));
            }
        }

        // 消息
        let kcp_data = Bytes::copy_from_slice(&segment[5..]);

        if let Ok(mut last_recv_time) = self.kcp_peer.last_recv_time.write() {
            *last_recv_time = self.kcp_peer.watch.elapsed();
        }

        // 根据通道类型处理消息
        match Kcp2KChannel::from(segment[0]) {
            Kcp2KChannel::Reliable => self.raw_input_reliable(kcp_data),
            Kcp2KChannel::Unreliable => self.raw_input_unreliable(kcp_data),
            _ => {
                self.on_error(ErrorCode::Unexpected, format!("{}: Received message with unexpected channel. Disconnecting the connection.", std::any::type_name::<Self>()));
                Err(ErrorCode::Unexpected)
            }
        }
    }
    fn receive_next_reliable(&self) -> Option<(Kcp2KHeaderReliable, Bytes)> {
        // 用于存储接收到的数据
        let mut buffer = BytesMut::new();
        // 初始化 buffer 大小
        if let Ok(kcp) = self.kcp_peer.kcp.read() {
            match kcp.peeksize() {
                Ok(size) => {
                    buffer.resize(size, 0);
                }
                Err(_) => {
                    return None;
                }
            }
        }
        // 从 KCP 接收数据
        if let Ok(mut kcp) = self.kcp_peer.kcp.write() {
            match kcp.recv(&mut buffer) {
                Ok(size) => {
                    if size == 0 {
                        self.on_error(
                            ErrorCode::InvalidReceive,
                            format!(
                                "{}: Receive failed with error={}. closing connection.",
                                std::any::type_name::<Self>(),
                                size
                            ),
                        );
                        self.send_disconnect();
                        return None;
                    }
                    // 解析头部
                    let header_byte = buffer[0];
                    match Kcp2KHeaderReliable::parse(header_byte) {
                        Some(header) => {
                            // 从 buffer 中提取消息
                            Some((header, Bytes::copy_from_slice(&buffer[1..size])))
                        }
                        None => {
                            self.on_error(ErrorCode::InvalidReceive, format!("[KCP-2K] {}: Receive failed to parse header: {} is not defined in {}.", std::any::type_name::<Self>(), header_byte, std::any::type_name::<Kcp2KHeaderReliable>()));
                            self.send_disconnect();
                            None
                        }
                    }
                }
                Err(error) => {
                    self.on_error(ErrorCode::InvalidReceive, format!("[KCP-2K] connection - {}: Receive failed with error={}. closing connection.", std::any::type_name::<Self>(), error));
                    self.send_disconnect();
                    None
                }
            }
        } else {
            None
        }
    }
    fn raw_input_reliable(&self, data: Bytes) -> Result<(), ErrorCode> {
        if let Ok(mut kcp) = self.kcp_peer.kcp.write() {
            if let Err(e) = kcp.input(&data) {
                self.on_error(
                    ErrorCode::InvalidReceive,
                    format!(
                        "[KCP2K] {}: Input failed with error={:?} for buffer with length={}",
                        std::any::type_name::<Self>(),
                        e,
                        data.len() - 1
                    ),
                );
                Err(ErrorCode::InvalidReceive)
            } else {
                Ok(())
            }
        } else {
            Err(ErrorCode::InvalidReceive)
        }
    }
    fn raw_input_unreliable(&self, data: Bytes) -> Result<(), ErrorCode> {
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
                self.on_disconnected();
                self.on_error(
                    ErrorCode::InvalidReceive,
                    format!(
                        "{}: Receive failed to parse header: {} is not defined in {}.",
                        std::any::type_name::<Self>(),
                        header,
                        std::any::type_name::<Kcp2KHeaderUnreliable>()
                    ),
                );
                return Err(ErrorCode::InvalidReceive);
            }
        };

        // 提取数据
        let data = Bytes::copy_from_slice(&data[1..]);

        // 根据头部类型处理消息
        match header {
            Kcp2KHeaderUnreliable::Data => match self.kcp_peer.state.try_read() {
                Ok(state) => match *state {
                    Kcp2KPeerState::Authenticated => {
                        self.on_data(data, Kcp2KChannel::Unreliable);
                        Ok(())
                    }
                    _ => {
                        self.on_error(ErrorCode::InvalidReceive, format!("{}: Received Data message while not Authenticated. Disconnecting the connection.", std::any::type_name::<Self>()));
                        Err(ErrorCode::InvalidReceive)
                    }
                },
                Err(err) => {
                    self.on_error(
                        ErrorCode::InvalidReceive,
                        format!(
                            "{}: Failed to read state: {}",
                            std::any::type_name::<Self>(),
                            err
                        ),
                    );
                    Err(ErrorCode::InvalidReceive)
                }
            },
            Kcp2KHeaderUnreliable::Disconnect => {
                self.on_disconnected();
                Ok(())
            }
            Kcp2KHeaderUnreliable::Ping => Ok(()),
        }
    }
    fn send_reliable(
        &self,
        kcp2k_header_reliable: Kcp2KHeaderReliable,
        data: Bytes,
    ) -> Result<(), ErrorCode> {
        // 创建一个缓冲区，用于存储消息内容
        let mut buffer = vec![];

        // 写入通道头部
        buffer.put_u8(kcp2k_header_reliable.to_u8());

        // 写入数据
        if !data.is_empty() {
            buffer.put_slice(&data);
        }

        // 通过 KCP 发送处理
        match self.kcp_peer.kcp.write() {
            Ok(mut kcp) => match kcp.send(&buffer) {
                Ok(_) => Ok(()),
                Err(e) => {
                    self.on_error(
                        ErrorCode::InvalidSend,
                        format!(
                            "{}: 发送失败，错误码={}，内容长度={}",
                            "send_reliable",
                            e,
                            data.len()
                        ),
                    );
                    Err(ErrorCode::SendError)
                }
            },
            Err(e) => {
                self.on_error(
                    ErrorCode::InvalidSend,
                    format!("{}: 发送失败，错误码={}", "send_reliable", e),
                );
                Err(ErrorCode::SendError)
            }
        }
    }
    fn send_unreliable(
        &self,
        kcp2k_header_unreliable: Kcp2KHeaderUnreliable,
        data: Bytes,
    ) -> Result<(), ErrorCode> {
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
        self.raw_send(&buffer)
    }
    pub fn tick_incoming(&self) {
        // 获取经过的时间
        let elapsed_time = self.kcp_peer.watch.elapsed();
        // 根据状态处理不同的逻辑
        let state = match self.kcp_peer.state.try_read() {
            Ok(state) => *state,
            Err(_) => {
                return;
            }
        };
        match state {
            Kcp2KPeerState::Connected => self.tick_incoming_connected(elapsed_time),
            Kcp2KPeerState::Authenticated => self.tick_incoming_authenticated(elapsed_time),
            Kcp2KPeerState::Disconnected => {}
        }
    }
    pub fn tick_outgoing(&self) {
        match self.kcp_peer.state.try_read() {
            Ok(state) => match *state {
                Kcp2KPeerState::Connected | Kcp2KPeerState::Authenticated => {
                    if let Ok(mut kcp) = self.kcp_peer.kcp.write() {
                        let _ = kcp.update(self.kcp_peer.watch.elapsed().as_millis() as u32);
                    }
                }
                Kcp2KPeerState::Disconnected => {}
            },
            Err(err) => {
                error!(format!(
                    "{}: Failed to read state: {}",
                    std::any::type_name::<Self>(),
                    err
                ));
            }
        }
    }
    // 获取地址
    pub fn get_sock_addr(&self) -> Arc<SockAddr> {
        Arc::clone(&self.client_sock_addr)
    }
    // 处理连接
    fn tick_incoming_connected(&self, elapsed_time: Duration) {
        self.handle_ping(elapsed_time);
        self.handle_timeout(elapsed_time);
        self.handle_dead_link();

        if let Some((header, _)) = self.receive_next_reliable() {
            match header {
                Kcp2KHeaderReliable::Hello => {
                    self.on_authenticated();
                }
                Kcp2KHeaderReliable::Data => {
                    self.on_error(
                        ErrorCode::InvalidReceive,
                        "Received invalid header while Connected. Disconnecting the connection."
                            .to_string(),
                    );
                    self.on_disconnected();
                }
                Kcp2KHeaderReliable::Ping => {}
            }
        }
    }
    // 处理认证
    fn tick_incoming_authenticated(&self, elapsed_time: Duration) {
        self.handle_ping(elapsed_time);
        self.handle_timeout(elapsed_time);
        self.handle_dead_link();

        if let Some((header, data)) = self.receive_next_reliable() {
            match header {
                Kcp2KHeaderReliable::Hello => {
                    self.on_error(ErrorCode::InvalidReceive, "Received invalid header while Authenticated. Disconnecting the connection.".to_string());
                    self.on_disconnected();
                }
                Kcp2KHeaderReliable::Data => {
                    if data.is_empty() {
                        self.on_error(ErrorCode::InvalidReceive, "Received empty Data message while Authenticated. Disconnecting the connection.".to_string());
                        self.on_disconnected();
                    } else {
                        self.on_data(data, Kcp2KChannel::Reliable);
                    }
                }
                Kcp2KHeaderReliable::Ping => {}
            }
        }
    }
    // 发送 hello
    fn send_hello(&self) {
        let _ = self.send_reliable(Kcp2KHeaderReliable::Hello, Default::default());
    }
    // 发送 ping
    fn send_ping(&self) {
        if self.is_reliable_ping {
            let _ = self.send_reliable(Kcp2KHeaderReliable::Ping, Default::default());
        } else {
            let _ = self.send_unreliable(Kcp2KHeaderUnreliable::Ping, Default::default());
        }
    }
    // 发送数据
    pub fn send_data(&mut self, data: Bytes, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        // 如果数据为空，则返回错误
        if data.is_empty() {
            self.on_error(
                ErrorCode::InvalidSend,
                "send_data: tried sending empty message. This should never happen. Disconnecting."
                    .to_string(),
            );
            return Err(ErrorCode::SendError);
        }
        // 根据通道类型发送数据
        match channel {
            Kcp2KChannel::Reliable => self.send_reliable(Kcp2KHeaderReliable::Data, data),
            Kcp2KChannel::Unreliable => self.send_unreliable(Kcp2KHeaderUnreliable::Data, data),
            _ => {
                self.on_error(ErrorCode::InvalidSend, format!("send_data: tried sending message with invalid channel: {:?}. Disconnecting.", channel));
                Err(ErrorCode::SendError)
            }
        }
    }
    // 发送断开连接
    pub fn send_disconnect(&self) {
        // 从连接列表中删除连接
        let _ = self.remove_connection_tx.send(self.connection_id);
        for _ in 0..5 {
            let _ = self.send_unreliable(Kcp2KHeaderUnreliable::Disconnect, Default::default());
        }
    }
    // 处理 ping
    fn handle_ping(&self, elapsed_time: Duration) {
        if let Ok(mut last_send_ping_time) = self.kcp_peer.last_send_ping_time.write() {
            if elapsed_time
                >= *last_send_ping_time + Duration::from_millis(Kcp2KConfig::PING_INTERVAL)
            {
                *last_send_ping_time = elapsed_time;
                self.send_ping();
            }
        }
    }
    // 处理超时
    fn handle_timeout(&self, elapsed_time: Duration) {
        if let Ok(last_recv_time) = self.kcp_peer.last_recv_time.read() {
            if elapsed_time > *last_recv_time + self.kcp_peer.timeout_duration {
                self.on_error(ErrorCode::Timeout, "timeout to disconnected.".to_string());
                self.on_disconnected();
            }
        }
    }
    // 处理 dead_link
    fn handle_dead_link(&self) {
        if let Ok(kcp) = self.kcp_peer.kcp.read() {
            if kcp.is_dead_link() {
                self.on_error(
                    ErrorCode::Timeout,
                    "dead link to disconnecting.".to_string(),
                );
                self.on_disconnected();
            }
        }
    }
}
