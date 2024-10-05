use crate::common;
use crate::common::Kcp2KMode;
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::ServerCallback;
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_connection::Kcp2KConnection;
use crate::kcp2k_header::Kcp2KHeaderReliable;
use crate::kcp2k_peer::Kcp2KPeer;
use bytes::Bytes;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::collections::HashMap;
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc, RwLock};
use tklog::{debug, info};

pub struct Client {
    config: Arc<Kcp2KConfig>,  // 配置
    socket: Arc<Socket>, // socket
    connections: HashMap<u64, Kcp2KConnection>,
    removed_connections: Arc<RwLock<Vec<u64>>>, // removed_connections
    callback_tx: Arc<mpsc::Sender<ServerCallback>>,
    client_model_default_connection_id: u64,
}

impl Client {
    pub fn new(config: Kcp2KConfig, addr: String) -> Result<(Self, mpsc::Receiver<ServerCallback>), Error> {
        let address: SocketAddr = addr.parse().unwrap();
        let socket = Socket::new(if config.dual_mode { Domain::IPV6 } else { Domain::IPV4 }, Type::DGRAM, Option::from(Protocol::UDP))?;
        common::configure_socket_buffers(&socket, config.recv_buffer_size, config.send_buffer_size, Arc::new(Kcp2KMode::Client))?;
        socket.set_nonblocking(true)?;
        socket.connect(&address.into())?;
        let (callback_tx, callback_rx) = mpsc::channel::<ServerCallback>();
        let mut instance = Client {
            config: Arc::new(config),
            socket: Arc::new(socket),
            connections: HashMap::new(),
            removed_connections: Arc::new(RwLock::new(Vec::new())),
            callback_tx: Arc::new(callback_tx),
            client_model_default_connection_id: rand::random(),
        };
        instance.create_connection(instance.client_model_default_connection_id, address.into());
        info!(format!("[KCP2K] Client connecting to: {:?}", instance.socket.peer_addr()?.as_socket().unwrap()));
        Ok((instance, callback_rx))
    }

    pub fn disconnect(&mut self) {
        self.connections.remove(&self.client_model_default_connection_id);
    }

    pub fn send(&mut self, data: Vec<u8>, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        if let Some(connection) = self.connections.get_mut(&self.client_model_default_connection_id) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }

    pub fn tick(&mut self) {
        self.tick_incoming();
        self.tick_outgoing();
    }

    fn raw_receive_from(&mut self) -> Option<(SockAddr, Bytes)> {
        let mut buf: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };

        match self.socket.recv_from(&mut buf) {
            Ok((size, sock_addr)) => {
                let buf = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len()) };
                Some((sock_addr, Bytes::copy_from_slice(&buf[..size])))
            }
            Err(_) => None
        }
    }
    fn create_connection(&mut self, connection_id: u64, sock_addr: SockAddr) {
        let kcp_client_connection = Kcp2KConnection::new(
            Arc::clone(&self.config),
            Arc::new(common::generate_cookie()),
            Arc::clone(&self.socket),
            connection_id,
            Arc::new(sock_addr),
            Arc::clone(&self.removed_connections),
            Arc::new(Kcp2KMode::Client),
            Arc::clone(&self.callback_tx),
        );
        self.connections.insert(connection_id, kcp_client_connection);
    }
    fn handle_data(&mut self, sock_addr: &SockAddr, data: Bytes) {
        // 生成连接 ID
        let connection_id = common::connection_hash(&sock_addr);
        // 如果连接存在，则处理数据
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            let _ = connection.raw_input(data);
        } else if data.len() > 28 && data[29] == Kcp2KHeaderReliable::Hello.to_u8() { // 如果是客户端模式
            let cookie = Bytes::copy_from_slice(&data[1..5]);
            debug!( format!("[KCP2K] Client received handshake with cookie={:?}", cookie.to_vec()));
            match self.connections.remove(&self.client_model_default_connection_id) {
                Some(mut conn) => {
                    self.client_model_default_connection_id = connection_id;
                    conn.set_connection_id(self.client_model_default_connection_id);
                    conn.set_kcp_peer(Kcp2KPeer::new(Arc::clone(&self.config), Arc::new(cookie), Arc::clone(&self.socket), Arc::new(sock_addr.clone())));
                    self.connections.insert(self.client_model_default_connection_id, conn);
                }
                None => {}
            }
        }
    }
    pub fn tick_incoming(&mut self) {
        while let Some((sock_addr, data)) = self.raw_receive_from() {
            self.handle_data(&sock_addr, data);
        }

        for connection in self.connections.values_mut() {
            connection.tick_incoming();
        }

        while let Some(connection_id) = self.removed_connections.write().unwrap().pop() {
            drop(self.connections.remove(&connection_id));
        }
    }
    pub fn tick_outgoing(&mut self) {
        for connection in self.connections.values_mut() {
            connection.tick_outgoing();
        }
    }
    pub fn get_connections(&self) -> &HashMap<u64, Kcp2KConnection> {
        &self.connections
    }
}
