use kcp2k_rust::kcp2k_callback::CallbackType;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;
use std::sync::Arc;

async fn update_several_times(amount: usize, server: &mut Server, interval: u64) {
    let interval = std::time::Duration::from_millis(interval);
    for _ in 0..amount {
        server.tick();
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


    // 启动服务器
    server.start().unwrap();
    // 服务器和客户端更新
    update_several_times(5, &mut server, config.interval).await;

    loop {
        server.tick();
        tokio::time::sleep(std::time::Duration::from_millis(config.interval)).await;
    }

    // let id = server.get_connections().keys().next().unwrap().clone();
    // server.s_send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    // update_several_times(10, &mut server, config.interval).await;
}

