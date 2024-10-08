use bytes::Bytes;
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let (server, s_rx) = Kcp2K::new_server(config, "0.0.0.0:3100".to_string()).unwrap();

    loop {
        // 服务器处理
        server.tick();
        // 服务器接收
        if let Ok(cb) = s_rx.try_recv() {
            match cb.callback_type {
                CallbackType::OnConnected => {
                    println!("Server OnConnected {:?}", cb.connection_id);
                    if let Err(e) = server.s_send(cb.connection_id, Bytes::from(vec![1, 2]), Kcp2KChannel::Reliable) {
                        println!("Server send error {:?}", e);
                    }
                }
                CallbackType::OnData => {
                    println!("Server received {:?} on channel {:?}", cb.data, cb.channel);
                    if let Err(e) = server.s_send(cb.connection_id, Bytes::from(vec![1, 2]), Kcp2KChannel::Reliable) {
                        println!("Server send error {:?}", e);
                    }
                }
                CallbackType::OnDisconnected => {
                    println!("Server OnDisconnected {}", cb.connection_id);
                }
                CallbackType::OnError => {
                    println!("Server OnError {} {}", cb.connection_id, cb.error_message);
                }
            }
        }
    }
}

