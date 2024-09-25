use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_client::Client;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;
use std::sync::Arc;

async fn update_several_times(amount: usize, server: &mut Server, client: &mut Client, interval: u64) {
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

    // 回调
    fn s_callback_fn(callback: kcp2k_rust::kcp2k_callback::Callback) {
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

    // 创建 KCP 服务器
    let mut server = Server::new(config, "0.0.0.0:3100".to_string(), Arc::new(s_callback_fn)).unwrap();


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

    // 启动服务器
    server.start().unwrap();
    // 启动客户端
    client.connect().unwrap();
    // 服务器和客户端更新
    update_several_times(5, &mut server, &mut client, config.interval).await;


    client.send(vec![1, 2], Kcp2KChannel::Reliable).unwrap();
    update_several_times(10, &mut server, &mut client, config.interval).await;

    let id = server.get_connections().keys().next().unwrap().clone();
    server.send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    update_several_times(10, &mut server, &mut client, config.interval).await;
}

