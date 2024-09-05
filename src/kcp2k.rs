use crate::common;
use crate::error_code::ErrorCode;
use crate::kcp2k_callback::Callback;
use crate::kcp2k_channel::Kcp2KChannel;
use crate::kcp2k_config::Kcp2KConfig;
use crate::kcp2k_peer::Kcp2KPeer;
use crate::kcp2k_server_connection::Kcp2KServerConnection;
use log::{error};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::cmp::PartialEq;
use std::collections::{HashMap};
use std::io::{Error};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time;
use tokio::sync::{mpsc};


#[derive(Debug, PartialEq)]
pub enum Kcp2KMode {
    Client,
    Server,
}

pub struct Kcp2K {
    config: Arc<Kcp2KConfig>,  // 配置
    socket: Arc<Socket>, // socket
    socket_addr: SockAddr, // socket_addr
    new_client_sock_addr: Arc<SockAddr>, // new_client_sock_addr
    connections: HashMap<u64, Kcp2KServerConnection>,
    removed_connections: Arc<Mutex<Vec<u64>>>, // removed_connections
    callback_tx: mpsc::UnboundedSender<Callback>,
    kcp2k_mode: Arc<Kcp2KMode>,
    client_model_default_connection_id: u64,
}


impl Kcp2K {
    pub fn new(config: Kcp2KConfig, addr: String, kcp2k_mode: Kcp2KMode) -> Result<(Self, mpsc::UnboundedReceiver<Callback>), Error> {
        let (callback_tx, callback_rx) = mpsc::unbounded_channel::<Callback>();

        let address: SocketAddr = addr.parse().unwrap();

        let domain = if config.dual_mode { Domain::IPV6 } else { Domain::IPV4 };
        let socket = Socket::new(domain, Type::DGRAM, Option::from(Protocol::UDP))?;
        socket.set_nonblocking(true)?;

        let instance = Kcp2K {
            config: Arc::new(config),
            socket: Arc::new(socket),
            socket_addr: address.into(),
            new_client_sock_addr: Arc::new(addr.parse::<SocketAddr>().map(SockAddr::from).unwrap()),
            connections: HashMap::new(),
            removed_connections: Arc::new(Mutex::new(Vec::new())),
            callback_tx,
            kcp2k_mode: Arc::new(kcp2k_mode),
            client_model_default_connection_id: rand::random(),
        };

        Ok((instance, callback_rx))
    }

    pub fn start(&mut self) -> Result<(), Error> {
        common::configure_socket_buffers(&self.socket, self.config.recv_buffer_size, self.config.send_buffer_size, Arc::clone(&self.kcp2k_mode))?;
        match self.kcp2k_mode.as_ref() {
            Kcp2KMode::Client => {
                println!("[KCP2K] {:?} connecting to: {:?}", self.kcp2k_mode, self.socket_addr.as_socket());
                self.socket.connect(&self.socket_addr)?;
                self.create_connection(self.client_model_default_connection_id);
            }
            Kcp2KMode::Server => {
                println!("[KCP2K] {:?} listening on: {:?}", self.kcp2k_mode, self.socket_addr.as_socket());
                self.socket.bind(&self.socket_addr)?;
            }
        }
        Ok(())
    }

    pub fn new_with_arc(config: Kcp2KConfig, addr: String, mode: Kcp2KMode) -> Result<(Arc<Mutex<Self>>, mpsc::UnboundedReceiver<Callback>), Error> {
        let (instance, callback_rx) = Self::new(config, addr, mode)?;
        Ok((Arc::new(Mutex::new(instance)), callback_rx))
    }

    pub fn loop_start(instance: &Arc<Mutex<Self>>) -> Result<(), Error> {
        instance.lock().unwrap().start()?;
        let interval = time::Duration::from_millis(instance.lock().unwrap().config.interval);
        loop {
            instance.lock().unwrap().tick();
            std::thread::sleep(interval);
        }
    }


    pub fn s_send(&mut self, connection_id: u64, data: Vec<u8>, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }

    pub fn c_send(&mut self, data: Vec<u8>, channel: Kcp2KChannel) -> Result<(), ErrorCode> {
        if let Some(connection) = self.connections.get_mut(&self.client_model_default_connection_id) {
            connection.send_data(data, channel)
        } else {
            Err(ErrorCode::ConnectionNotFound)
        }
    }


    pub fn tick(&mut self) {
        self.tick_incoming();
        self.tick_outgoing();
        self.tick_connections();
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
            Err(e) => {
                error!("recv_from failed: {:?}", e);
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
            self.callback_tx.clone(),
            Arc::clone(&self.removed_connections),
            Arc::clone(&self.kcp2k_mode),
        );
        self.connections.insert(connection_id, kcp_server_connection);
    }
    fn handle_data(&mut self, data: Vec<u8>, connection_id: u64) {
        // 如果连接存在，则处理数据
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            let _ = connection.on_raw_input(data);
        } else if self.kcp2k_mode == Arc::from(Kcp2KMode::Client) { // 如果是客户端模式
            let mut cookie = common::generate_cookie();
            if data.len() > 4 {
                cookie = data[1..5].to_vec();
                println!("[KCP2K] {:?} received handshake with cookie={:?}", self.kcp2k_mode, cookie);
            }
            match self.connections.remove(&self.client_model_default_connection_id) {
                Some(mut conn) => {
                    self.client_model_default_connection_id = connection_id;
                    conn.set_connection_id(connection_id);
                    conn.set_kcp_peer(Kcp2KPeer::new(Arc::clone(&self.config), Arc::new(cookie), Arc::clone(&self.socket), Arc::clone(&self.new_client_sock_addr)));
                    self.connections.insert(connection_id, conn);
                    self.handle_data(data, connection_id);
                }
                None => {}
            }
        } else if self.kcp2k_mode == Arc::from(Kcp2KMode::Server) { // 如果是服务器模式
            self.create_connection(connection_id);
            self.handle_data(data, connection_id);
        }
    }
    fn tick_incoming(&mut self) {
        while let Some((connection_id, data)) = self.raw_receive_from() {
            self.handle_data(data, connection_id);
        }

        for connection in self.connections.values_mut() {
            connection.tick_incoming();
        }
    }
    fn tick_outgoing(&mut self) {
        for connection in self.connections.values_mut() {
            connection.tick_outgoing();
        }
    }
    fn tick_connections(&mut self) {
        while let Some(connection_id) = self.removed_connections.lock().unwrap().pop() {
            drop(self.connections.remove(&connection_id));
        }
    }
    pub fn get_connections(&self) -> &HashMap<u64, Kcp2KServerConnection> {
        &self.connections
    }
}
