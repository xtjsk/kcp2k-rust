use bytes::Bytes;
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use std::process::exit;

fn main() {
    // 创建 KCP 客户端配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 客户端
    let (client, c_rx) = Kcp2K::new_client(config, "127.0.0.1:7777".to_string()).unwrap();

    loop {
        // 客户端处理
        client.tick();
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
                    exit(0);
                }
                CallbackType::OnError => {
                    println!("Client OnError {:?} {}", cb.connection_id, cb.error_message);
                }
            }
        }
    }
}
