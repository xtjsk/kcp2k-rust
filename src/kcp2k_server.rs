use crate::common;
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::Callback;
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_server_connection::Kcp2KServerConnection;
use common::Kcp2KMode;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::collections::HashMap;
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tklog::info;

pub struct Server {
    config: Arc<Kcp2KConfig>,  // 配置
    socket: Arc<Socket>, // socket
    socket_addr: SockAddr, // socket_addr
    new_client_sock_addr: Arc<SockAddr>, // new_client_sock_addr
    connections: HashMap<u64, Kcp2KServerConnection>,
    removed_connections: Arc<Mutex<Vec<u64>>>, // removed_connections
    callback_fn: Arc<dyn Fn(&Callback) + Send>,
}


impl Server {
    pub fn new(config: Kcp2KConfig, addr: String, callback_fn: Arc<dyn Fn(&Callback) + Send>) -> Result<Self, Error> {
        let address: SocketAddr = addr.parse().unwrap();

        let domain = if config.dual_mode { Domain::IPV6 } else { Domain::IPV4 };
        let socket = Socket::new(domain, Type::DGRAM, Option::from(Protocol::UDP))?;
        socket.set_nonblocking(true)?;

        let instance = Server {
            config: Arc::new(config),
            socket: Arc::new(socket),
            socket_addr: address.into(),
            new_client_sock_addr: Arc::new(addr.parse::<SocketAddr>().map(SockAddr::from).unwrap()),
            connections: HashMap::new(),
            removed_connections: Arc::new(Mutex::new(Vec::new())),
            callback_fn,
        };

        Ok(instance)
    }
    pub fn start(&mut self) -> Result<(), Error> {
        common::configure_socket_buffers(&self.socket, self.config.recv_buffer_size, self.config.send_buffer_size, Arc::new(Kcp2KMode::Server))?;
        info!(format!("[KCP2K] Server listening on: {:?}", self.socket_addr.as_socket()));
        self.socket.bind(&self.socket_addr)?;

        Ok(())
    }
    pub fn stop(&mut self) {
        self.socket.shutdown(std::net::Shutdown::Both).unwrap();
    }
    pub fn send(&mut self, connection_id: u64, data: Vec<u8>, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }
    pub fn tick(&mut self) {
        self.tick_incoming();
        self.tick_outgoing();
    }
    fn raw_receive_from(&mut self) -> Option<(u64, Vec<u8>)> {
        let mut buf: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };

        match self.socket.recv_from(&mut buf) {
            Ok((size, client_addr)) => {
                self.new_client_sock_addr = Arc::new(client_addr);
                let buf: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len()) };
                let id = common::connection_hash(&self.new_client_sock_addr);
                Some((id, buf[..size].to_vec()))
            }
            Err(_) => {
                None
            }
        }
    }
    fn create_connection(&mut self, connection_id: u64) {
        let kcp_server_connection = Kcp2KServerConnection::new(
            Arc::clone(&self.config),
            Arc::new(common::generate_cookie()),
            Arc::clone(&self.socket),
            connection_id,
            Arc::clone(&self.new_client_sock_addr),
            Arc::clone(&self.removed_connections),
            Arc::new(Kcp2KMode::Server),
            Arc::clone(&self.callback_fn),
        );
        self.connections.insert(connection_id, kcp_server_connection);
    }
    fn handle_data(&mut self, connection_id: u64, data: Vec<u8>) {
        // 如果连接存在，则处理数据
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            let _ = connection.raw_input(data);
        } else { // 如果连接不存在，则创建连接
            self.create_connection(connection_id);
            self.handle_data(connection_id, data);
        }
    }
    pub fn tick_incoming(&mut self) {
        while let Some((connection_id, data)) = self.raw_receive_from() {
            self.handle_data(connection_id, data);
        }

        for connection in self.connections.values_mut() {
            connection.tick_incoming();
        }

        while let Some(connection_id) = self.removed_connections.lock().unwrap().pop() {
            drop(self.connections.remove(&connection_id));
        }
    }
    pub fn tick_outgoing(&mut self) {
        for connection in self.connections.values_mut() {
            connection.tick_outgoing();
        }
    }
    pub fn get_connections(&self) -> &HashMap<u64, Kcp2KServerConnection> {
        &self.connections
    }
}
