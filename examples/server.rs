use kcp2k_rust::kcp2k_callback::ServerCallbackType;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;
use std::sync::Arc;

fn update_several_times(amount: usize, server: &mut Server) {
    for _ in 0..amount {
        server.tick();
    }
}

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 回调
    fn s_callback_fn(callback: kcp2k_rust::kcp2k_callback::ServerCallback) {
        match callback.callback_type {
            ServerCallbackType::OnConnected => {
                println!("server OnConnected: {:?}", callback)
            }
            ServerCallbackType::OnData => {
                println!("server OnData: {:?}", callback);
            }
            ServerCallbackType::OnError => {
                println!("server OnError: {:?}", callback);
            }
            ServerCallbackType::OnDisconnected => {
                println!("server on_disconnected: {:?}", callback);
            }
        }
    }

    // 创建 KCP 服务器
    let mut server = Server::new(config, "0.0.0.0:3100".to_string(), Arc::new(s_callback_fn)).unwrap();

    // 服务器和客户端更新
    update_several_times(5, &mut server);

    loop {
        server.tick();
    }

    // let id = server.get_connections().keys().next().unwrap().clone();
    // server.s_send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    // update_several_times(10, &mut server, config.interval).await;
}

