use crate::common;
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::Callback;
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_connection::Kcp2KConnection;
use crate::kcp2k_header::Kcp2KHeaderReliable;
use crate::kcp2k_peer::Kcp2KPeer;
use bytes::Bytes;
use common::Kcp2KMode;
use crossbeam_channel;
use dashmap::mapref::one::RefMut;
use dashmap::try_result::TryResult;
use dashmap::DashMap;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tklog::{debug, info, warn};

pub struct Kcp2K {
    mode: Kcp2KMode,
    config: Arc<Kcp2KConfig>, // 配置
    socket: Arc<Socket>,      // socket
    connections: DashMap<u64, Kcp2KConnection>,
    callback: fn(Callback),
    remove_connection_tx: Arc<crossbeam_channel::Sender<u64>>,
    remove_connection_rx: Arc<crossbeam_channel::Receiver<u64>>,
    client_model_default_connection_id: AtomicU64,
}

impl Kcp2K {
    pub fn new_server(
        config: Kcp2KConfig,
        addr: String,
        callback: fn(Callback),
    ) -> Result<Self, Error> {
        let socket_addr: SocketAddr = addr.parse().unwrap();
        let socket = Socket::new(
            if config.dual_mode {
                Domain::IPV6
            } else {
                Domain::IPV4
            },
            Type::DGRAM,
            Option::from(Protocol::UDP),
        )?;
        common::configure_socket_buffers(
            &socket,
            config.recv_buffer_size,
            config.send_buffer_size,
            Arc::new(Kcp2KMode::Server),
        )?;
        socket.set_nonblocking(true)?;
        socket.bind(&socket_addr.into())?;
        let (remove_connection_tx, remove_connection_rx) = crossbeam_channel::unbounded::<u64>();
        let server = Self::new(
            config,
            Kcp2KMode::Server,
            socket,
            callback,
            remove_connection_tx,
            remove_connection_rx,
        );
        info!(format!(
            "[KCP2K] Server bind on: {:?}",
            server.socket.local_addr()?.as_socket().unwrap()
        ));
        Ok(server)
    }
    pub fn new_client(
        config: Kcp2KConfig,
        addr: String,
        callback: fn(Callback),
    ) -> Result<Self, Error> {
        let address: SocketAddr = addr.parse().unwrap();
        let socket = Socket::new(
            if config.dual_mode {
                Domain::IPV6
            } else {
                Domain::IPV4
            },
            Type::DGRAM,
            Option::from(Protocol::UDP),
        )?;
        common::configure_socket_buffers(
            &socket,
            config.recv_buffer_size,
            config.send_buffer_size,
            Arc::new(Kcp2KMode::Client),
        )?;
        socket.set_nonblocking(true)?;
        socket.connect(&address.into())?;
        let (remove_connection_tx, remove_connection_rx) = crossbeam_channel::unbounded::<u64>();
        let client = Self::new(
            config,
            Kcp2KMode::Client,
            socket,
            callback,
            remove_connection_tx,
            remove_connection_rx,
        );
        client.create_connection(
            client
                .client_model_default_connection_id
                .load(Ordering::SeqCst),
            address.into(),
        );
        info!(format!(
            "[KCP2K] Client connecting to: {:?}",
            client.socket.peer_addr()?.as_socket().unwrap()
        ));
        Ok(client)
    }
    fn new(
        config: Kcp2KConfig,
        mode: Kcp2KMode,
        socket: Socket,
        callback: fn(Callback),
        remove_connection_tx: crossbeam_channel::Sender<u64>,
        remove_connection_rx: crossbeam_channel::Receiver<u64>,
    ) -> Self {
        Self {
            mode,
            config: Arc::new(config),
            socket: Arc::new(socket),
            connections: DashMap::new(),
            callback,
            remove_connection_tx: Arc::new(remove_connection_tx),
            remove_connection_rx: Arc::new(remove_connection_rx),
            client_model_default_connection_id: AtomicU64::new(rand::random()),
        }
    }
    pub fn stop(&self) -> Result<(), Error> {
        match self.socket.shutdown(std::net::Shutdown::Both) {
            Ok(_) => {
                self.connections.clear();
                warn!("[KCP2K] Stopped".to_string());
                Ok(())
            }
            Err(_) => {
                warn!("[KCP2K] Failed to stop KCP2K".to_string());
                Err(Error::from_raw_os_error(1))
            }
        }
    }
    pub fn s_send(
        &self,
        connection_id: u64,
        data: Bytes,
        channel: Kcp2KChannel,
    ) -> Result<(), ErrorCode> {
        if let Some(mut connection) = self.connections.get_mut(&connection_id) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }
    pub fn c_send(&self, data: Bytes, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        if let Some(mut connection) = self.connections.get_mut(
            &self
                .client_model_default_connection_id
                .load(Ordering::SeqCst),
        ) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }
    fn raw_receive_from(&self) -> Option<(SockAddr, Bytes)> {
        let mut buf: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
        match self.socket.recv_from(&mut buf) {
            Ok((size, sock_addr)) => {
                let buf = unsafe {
                    std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len())
                };
                Some((sock_addr, Bytes::copy_from_slice(&buf[..size])))
            }
            Err(_) => None,
        }
    }
    fn handle_data(&self, sock_addr: &SockAddr, data: Bytes) {
        // 生成连接 ID
        let connection_id = common::connection_hash(sock_addr);
        // 如果连接存在，则处理数据
        if let Some(mut connection) = self.connections.get_mut(&connection_id) {
            let _ = connection.raw_input(data);
        } else if self.mode == Kcp2KMode::Server {
            // 如果连接不存在，则创建连接
            self.create_connection(connection_id, sock_addr.clone());
        } else if self.mode == Kcp2KMode::Client
            && data.len() > 28
            && data[29] == Kcp2KHeaderReliable::Hello.to_u8()
        {
            // 如果是客户端模式
            let cookie = Bytes::copy_from_slice(&data[1..5]);
            debug!(format!(
                "[KCP2K] Client received handshake with cookie={:?}",
                cookie.to_vec()
            ));
            match self.connections.remove(
                &self
                    .client_model_default_connection_id
                    .load(Ordering::SeqCst),
            ) {
                Some((_, mut conn)) => {
                    self.client_model_default_connection_id
                        .store(connection_id, Ordering::SeqCst);
                    conn.set_connection_id(
                        self.client_model_default_connection_id
                            .load(Ordering::SeqCst),
                    );
                    conn.set_kcp_peer(Kcp2KPeer::new(
                        Arc::new(self.mode),
                        Arc::clone(&self.config),
                        Arc::new(cookie),
                        Arc::clone(&self.socket),
                        Arc::new(sock_addr.clone()),
                    ));
                    self.connections.insert(
                        self.client_model_default_connection_id
                            .load(Ordering::SeqCst),
                        conn,
                    );
                }
                None => {}
            }
        }
    }
    fn create_connection(&self, connection_id: u64, sock_addr: SockAddr) {
        let cookie = common::generate_cookie();
        debug!(format!(
            "[KCP2K] Created connection {} with cookie {:?}",
            connection_id,
            cookie.to_vec()
        ));
        let kcp_server_connection = Kcp2KConnection::new(
            Arc::clone(&self.config),
            Arc::new(cookie),
            Arc::clone(&self.socket),
            connection_id,
            Arc::new(sock_addr),
            Arc::new(self.mode),
            self.callback,
            Arc::clone(&self.remove_connection_tx),
        );

        self.connections
            .insert(connection_id, kcp_server_connection);
    }
    pub fn tick(&self) {
        self.tick_incoming();
        self.tick_outgoing();
    }
    pub fn tick_incoming(&self) {
        while let Ok(connection_id) = self.remove_connection_rx.try_recv() {
            self.connections.remove(&connection_id);
        }

        while let Some((sock_addr, data)) = self.raw_receive_from() {
            self.handle_data(&sock_addr, data);
        }

        for connection in self.connections.iter() {
            connection.tick_incoming();
        }
    }
    pub fn tick_outgoing(&self) {
        for connection in self.connections.iter() {
            connection.tick_outgoing();
        }
    }
    pub fn get_connection(&self, connection_id: u64) -> Option<RefMut<u64, Kcp2KConnection>> {
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            Some(connection)
        } else {
            None
        }
    }
    pub fn get_connection_address(&self, connection_id: u64) -> String {
        if let Some(connection) = self.connections.get(&connection_id) {
            if let Some(sock_addr) = connection.get_sock_addr().as_socket() {
                return sock_addr.to_string();
            }
        };
        "".to_string()
    }
    pub fn get_connections(&self) -> &DashMap<u64, Kcp2KConnection> {
        &self.connections
    }
    pub fn close_connection(&self, connection_id: u64) {
        match self.connections.try_get(&connection_id) {
            TryResult::Present(conn) => {
                conn.send_disconnect();
            }
            TryResult::Absent => {
                warn!(format!("[KCP2K] Connection {} not found", connection_id));
            }
            TryResult::Locked => {
                warn!(format!("[KCP2K] Connection {} is locked", connection_id));
            }
        }
    }
}
