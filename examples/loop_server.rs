use kcp2k_rust::kcp2k::{Kcp2K, Kcp2KMode};
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

#[tokio::main]
async fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let (server, mut callback_rx) = Kcp2K::new_with_arc(config, "0.0.0.0:3100".to_string(), Kcp2KMode::Server).unwrap();

    // 服务器克隆
    let serv_c = server.clone();

    // 服务器回调处理
    tokio::spawn(async move {
        while let Some(callback) = callback_rx.recv().await {
            match callback.callback_type {
                CallbackType::OnConnected => {
                    println!("server OnConnected: {:?}", callback);
                }
                CallbackType::OnData => {
                    println!("server OnData: {:?}", callback);
                    // let p = Packet::deserialize2struct(&callback.data);
                    // println!("packet: {:?}", p);
                    // let packet = Packet::new(callback.connection_id, callback.channel as u8, callback.data);
                    let _ = serv_c.lock().unwrap().s_send(callback.connection_id, callback.data, callback.channel);
                }
                CallbackType::OnError => {
                    println!("server OnError: {:?}", callback);
                }
                CallbackType::OnDisconnected => {
                    println!("server on_disconnected: {:?}", callback);
                }
            }
        }
    });

    // 启动服务器
    if let Err(e) = Kcp2K::loop_start(&server) {
        println!("Server start failed: {:?}", e);
    }
}

