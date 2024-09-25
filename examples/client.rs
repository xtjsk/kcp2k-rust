use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_client::Client;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use std::sync::Arc;

async fn update_several_times(amount: usize, client: &mut Client, interval: u64) {
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

    // 回调
    fn c_callback_fn(callback: kcp2k_rust::kcp2k_callback::Callback) {
        match callback.callback_type {
            CallbackType::OnConnected => {
                println!("client OnConnected: {:?}", callback)
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

    // 创建 KCP 客户端
    let mut client = Client::new(config, "127.0.0.1:3100".to_string(), Arc::new(c_callback_fn)).unwrap();

    // 启动客户端
    client.connect().unwrap();
    // 客户端更新
    update_several_times(5, &mut client, config.interval).await;

    loop {
        if let Err(e) = client.send(vec![1, 2], Kcp2KChannel::Reliable) {
            println!("client send failed: {:?}", e);
            break;
        }
        update_several_times(10, &mut client, config.interval).await;
    }
}

