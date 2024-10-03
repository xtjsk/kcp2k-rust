use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let (mut server, s_rx) = Server::new(config, "0.0.0.0:3100".to_string()).unwrap();


    loop {
        if let Ok(callback) = s_rx.try_recv() {
            println!("{:?}", callback);
        }
        server.tick();
    }

    // let id = server.get_connections().keys().next().unwrap().clone();
    // server.s_send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    // update_several_times(10, &mut server, config.interval).await;
}

