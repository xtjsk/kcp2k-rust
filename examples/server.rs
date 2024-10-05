use kcp2k_rust::kcp2k_callback::ServerCallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let (mut server, s_rx) = Server::new(config, "0.0.0.0:3100".to_string()).unwrap();

    loop {
        // 服务器处理
        server.tick();
        // 服务器接收
        if let Ok(cb) = s_rx.try_recv() {
            match cb.callback_type {
                ServerCallbackType::OnConnected => {
                    println!("Server OnConnected {:?}", cb.connection_id);
                    if let Err(e) = server.send(cb.connection_id, vec![1, 2], Kcp2KChannel::Reliable) {
                        println!("Server send error {:?}", e);
                    }
                }
                ServerCallbackType::OnData => {
                    println!("Server received {:?} on channel {:?}", cb.data, cb.channel);
                    if let Err(e) = server.send(cb.connection_id, vec![1, 2], Kcp2KChannel::Reliable){
                        println!("Server send error {:?}", e);
                    }
                }
                ServerCallbackType::OnDisconnected => {
                    println!("Server OnDisconnected {}", cb.connection_id);
                }
                ServerCallbackType::OnError => {
                    println!("Server OnError {} {}", cb.connection_id, cb.error_message);
                }
            }
        }
    }
}

