use bytes::BytesMut;
use kcp2k_rust::kcp2k::{Kcp2K, Kcp2KMode};
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::packet::Packet;

#[tokio::main]
async fn main() {
    // 创建 KCP 客户端配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 客户端
    let (client, mut callback_rx) = Kcp2K::new_with_arc(config, "127.0.0.1:3100".to_string(), Kcp2KMode::Client).unwrap();

    // 客户端克隆
    let client_c = client.clone();

    // 服务器回调处理
    tokio::spawn(async move {
        while let Some(callback) = callback_rx.recv().await {
            match callback.callback_type {
                CallbackType::OnConnected => {
                    println!("client OnConnected: {:?}", callback);
                    let packet = Packet::new(callback.connection_id, callback.channel as u8, BytesMut::from("Hello, KCP!").to_vec());
                    let _ = client_c.lock().unwrap().s_send(callback.connection_id, packet.serialize(), Kcp2KChannel::Reliable);
                }
                CallbackType::OnData => {
                    println!("client OnData: {:?}", callback);
                    let p = Packet::deserialize2struct(&callback.data);
                    println!("packet: {:?}", p);
                    println!("data: {}", Packet::vec_u8_to_string(&p.data));
                    let packet = Packet::new(callback.connection_id, callback.channel as u8, p.data);
                    client_c.lock().unwrap().s_send(callback.connection_id, packet.serialize(), callback.channel).expect("TODO: panic message");
                }
                CallbackType::OnError => {
                    println!("client OnError: {:?}", callback);
                }
                CallbackType::OnDisconnected => {
                    println!("client on_disconnected: {:?}", callback);
                }
            }
        }
    });

    // 启动服务器
    if let Err(e) = Kcp2K::loop_start(&client) {
        println!("Client start failed: {:?}", e);
    }
}

