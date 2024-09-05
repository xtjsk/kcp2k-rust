use kcp2k_rust::kcp2k::{Kcp2K, Kcp2KMode};
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;


async fn update_several_times(amount: usize, server: &mut Kcp2K, client: &mut Kcp2K, interval: u64) {
    let interval = std::time::Duration::from_millis(interval);
    for _ in 0..amount {
        server.tick();
        client.tick();
        tokio::time::sleep(interval).await;
    }
}

#[tokio::main]
async fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let (mut server, mut callback_rx) = Kcp2K::new(config, "0.0.0.0:3100".to_string(), Kcp2KMode::Server).unwrap();

    // 服务器回调处理
    tokio::spawn(async move {
        while let Some(callback) = callback_rx.recv().await {
            match callback.callback_type {
                CallbackType::OnConnected => {
                    println!("server OnConnected: {:?}", callback)
                }
                CallbackType::OnData => {
                    println!("server OnData: {:?}", callback);
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


    // 创建 KCP 客户端
    let (mut client, mut callback_rx) = Kcp2K::new(config, "127.0.0.1:3100".to_string(), Kcp2KMode::Client).unwrap();


    // 客户端回调处理
    tokio::spawn(async move {
        while let Some(callback) = callback_rx.recv().await {
            match callback.callback_type {
                CallbackType::OnConnected => {
                    println!("client OnConnected: {:?}", callback);
                }
                CallbackType::OnData => {
                    println!("client OnData: {:?}", callback);
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
    server.start().unwrap();
    // 启动客户端
    client.start().unwrap();
    // 服务器和客户端更新
    update_several_times(5, &mut server, &mut client, config.interval).await;


    client.c_send(vec![1, 2], Kcp2KChannel::Reliable).unwrap();
    update_several_times(10, &mut server, &mut client, config.interval).await;

    let id = server.get_connections().keys().next().unwrap().clone();
    server.s_send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    update_several_times(10, &mut server, &mut client, config.interval).await;
}

