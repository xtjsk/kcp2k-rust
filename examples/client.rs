use kcp2k_rust::error_code::ErrorCode;
use kcp2k_rust::kcp2k::{Kcp2K, Kcp2KMode};
use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

async fn update_several_times(amount: usize, client: &mut Kcp2K, interval: u64) {
    let interval = std::time::Duration::from_millis(interval);
    for _ in 0..amount {
        client.tick();
        tokio::time::sleep(interval).await;
    }
}

#[tokio::main]
async fn main() {
    // 创建 KCP 客户端配置
    let config = Kcp2KConfig::default();

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

    // 启动客户端
    client.start().unwrap();
    // 客户端更新
    update_several_times(5, &mut client, config.interval).await;

    loop {
        if let Err(e) = client.c_send(vec![1, 2], Kcp2KChannel::Reliable) {
            println!("client send failed: {:?}", e);
        }
        update_several_times(10, &mut client, config.interval).await;
    }
}

