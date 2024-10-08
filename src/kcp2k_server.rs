use crate::common;
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::Callback;
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_connection::Kcp2KConnection;
use bytes::Bytes;
use common::Kcp2KMode;
use dashmap::DashMap;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use tklog::info;

pub struct Server {
    config: Arc<Kcp2KConfig>,  // 配置
    socket: Arc<Socket>, // socket
    connections: Arc<DashMap<u64, Kcp2KConnection>>,
    callback_tx: Arc<mpsc::Sender<Callback>>,
    remove_connection_tx: Arc<mpsc::Sender<u64>>,
    remove_connection_rx: Arc<mpsc::Receiver<u64>>,
}

impl Server {
    pub fn new(config: Kcp2KConfig, addr: String) -> Result<(Self, mpsc::Receiver<Callback>), Error> {
        let socket_addr: SocketAddr = addr.parse().unwrap();
        let socket = Socket::new(if config.dual_mode { Domain::IPV6 } else { Domain::IPV4 }, Type::DGRAM, Option::from(Protocol::UDP))?;
        common::configure_socket_buffers(&socket, config.recv_buffer_size, config.send_buffer_size, Arc::new(Kcp2KMode::Server))?;
        socket.set_nonblocking(true)?;
        socket.bind(&socket_addr.into())?;
        let (callback_tx, callback_rx) = mpsc::channel::<Callback>();
        let (remove_connection_tx, remove_connection_rx) = mpsc::channel::<u64>();
        let instance = Server {
            config: Arc::new(config),
            socket: Arc::new(socket),
            connections: Arc::new(DashMap::new()),
            callback_tx: Arc::new(callback_tx),
            remove_connection_tx: Arc::new(remove_connection_tx),
            remove_connection_rx: Arc::new(remove_connection_rx),
        };
        info!(format!("[KCP2K] Server bind on: {:?}", instance.socket.local_addr()?.as_socket().unwrap()));
        Ok((instance, callback_rx))
    }
    pub fn stop(&self) -> Result<(), Error> {
        match self.socket.shutdown(std::net::Shutdown::Both) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::from_raw_os_error(0))
        }
    }
    pub fn send(&self, connection_id: u64, data: Bytes, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        if let Some(mut connection) = self.connections.get_mut(&connection_id) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }
    fn raw_receive_from(&self) -> Option<(SockAddr, Bytes)> {
        let mut buf: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
        match self.socket.recv_from(&mut buf) {
            Ok((size, sock_addr)) => {
                let buf = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len()) };
                Some((sock_addr, Bytes::copy_from_slice(&buf[..size])))
            }
            Err(_) => None
        }
    }
    fn handle_data(&self, sock_addr: &SockAddr, data: Bytes) {
        // 生成连接 ID
        let connection_id = common::connection_hash(sock_addr);
        // 如果连接存在，则处理数据
        if let Some(mut connection) = self.connections.get_mut(&connection_id) {
            let _ = connection.raw_input(data);
        } else { // 如果连接不存在，则创建连接
            self.create_connection(connection_id, sock_addr.clone());
        }
    }
    fn create_connection(&self, connection_id: u64, sock_addr: SockAddr) {
        let kcp_server_connection = Kcp2KConnection::new(
            Arc::clone(&self.config),
            Arc::new(common::generate_cookie()),
            Arc::clone(&self.socket),
            connection_id,
            Arc::new(sock_addr),
            Arc::new(Kcp2KMode::Server),
            Arc::clone(&self.callback_tx),
            Arc::clone(&self.remove_connection_tx),
        );
        self.connections.insert(connection_id, kcp_server_connection);
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
    pub fn get_connection(&self, connection_id: u64) -> Option<Kcp2KConnection> {
        if let Some(connection) = self.connections.get(&connection_id) {
            Some(connection.clone())
        } else {
            None
        }
    }
    pub fn get_connections(&self) -> Arc<DashMap<u64, Kcp2KConnection>> {
        Arc::clone(&self.connections)
    }
}
