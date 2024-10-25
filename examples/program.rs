use bytes::Bytes;
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use std::thread::sleep;

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let (server, s_rx) = Kcp2K::new_server(config, "0.0.0.0:3100".to_string()).unwrap();

    // 创建 KCP 客户端
    let (client, c_rx) = Kcp2K::new_client(config, "127.0.0.1:3100".to_string()).unwrap();

    loop {
        // 服务器处理
        server.tick();
        // 客户端处理
        client.tick();
        // 服务器接收
        if let Ok(cb) = s_rx.try_recv() {
            match cb.callback_type {
                CallbackType::OnConnected => {
                    println!("Server OnConnected {:?}", cb.connection_id);
                    if let Err(e) = server.s_send(
                        cb.connection_id,
                        Bytes::from(vec![1, 2]),
                        Kcp2KChannel::Reliable,
                    ) {
                        println!("Server send error {:?}", e);
                    }
                }
                CallbackType::OnData => {
                    println!("{}", server.get_connection_address(cb.connection_id));
                    println!(
                        "Server received {:?} on channel {:?}",
                        cb.data.as_ref(),
                        cb.channel
                    );
                    if let Err(e) = server.s_send(
                        cb.connection_id,
                        Bytes::from(vec![1, 2]),
                        Kcp2KChannel::Reliable,
                    ) {
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
        // 客户端接收
        if let Ok(cb) = c_rx.try_recv() {
            match cb.callback_type {
                CallbackType::OnConnected => {
                    println!("Client OnConnected {}", cb.connection_id);
                }
                CallbackType::OnData => {
                    println!(
                        "Client received {:?} on channel {:?}",
                        cb.data.as_ref(),
                        cb.channel
                    );
                    if let Err(e) = client.c_send(Bytes::from(vec![3, 4]), Kcp2KChannel::Unreliable)
                    {
                        println!("Client send error {:?}", e);
                    }
                }
                CallbackType::OnDisconnected => {
                    println!("Client OnDisconnected {}", cb.connection_id);
                }
                CallbackType::OnError => {
                    println!("Client OnError {:?} {}", cb.connection_id, cb.error_message);
                }
            }
        }
        sleep(std::time::Duration::from_millis(10));
    }
}
