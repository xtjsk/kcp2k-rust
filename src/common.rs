use rand::TryRngCore;
use socket2::{SockAddr, Socket};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Error;
use std::sync::Arc;
use crate::kcp2k::Kcp2KMode;

// sock_addr hash
pub fn connection_hash(sock_addr: &SockAddr) -> u64 {
    let mut hasher = DefaultHasher::new();
    sock_addr.hash(&mut hasher);
    hasher.finish()
}


// 生成一个随机的 4 字节 cookie
pub fn generate_cookie() -> Vec<u8> {
    let mut rng = rand::rngs::OsRng;
    let mut buffer = [0u8; 4];
    let _ = rng.try_fill_bytes(&mut buffer);
    buffer.to_vec()
}

// 如果连接在重负载下下降，请增加到 OS 限制。
// 如果仍然不够，请增加 OS 限制。
pub fn configure_socket_buffers(socket: &Socket, recv_buffer_size: usize, send_buffer_size: usize, kcp2k_mode: Arc<Kcp2KMode>) -> Result<(), Error> {
    // 记录初始大小以进行比较
    let initial_receive = socket.recv_buffer_size()?;
    let initial_send = socket.send_buffer_size()?;

    // 设置为配置的大小
    socket.set_recv_buffer_size(recv_buffer_size)?;
    socket.set_send_buffer_size(send_buffer_size)?;

    println!("[KCP2K] {:?} RecvBuf = {}=>{} ({}x) SendBuf = {}=>{} ({}x)",
             kcp2k_mode,
             initial_receive, socket.recv_buffer_size()?, socket.recv_buffer_size()? / initial_receive,
             initial_send, socket.send_buffer_size()?, socket.send_buffer_size()? / initial_send);
    Ok(())
}