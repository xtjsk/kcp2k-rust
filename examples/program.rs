use kcp2k_rust::common::{update_client_times, update_times};
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_client::Client;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();


    // 创建 KCP 服务器
    let (mut server, s_rx) = Server::new(config, "0.0.0.0:3100".to_string()).unwrap();


    // 创建 KCP 客户端
    let (mut client, c_rx) = Client::new(config, "127.0.0.1:3100".to_string()).unwrap();

    update_times(10, &mut client, &mut server, config.interval);

    client.send(vec![1, 2], Kcp2KChannel::Reliable).unwrap();
    update_client_times(5, &mut client, config.interval);


    let id = server.get_connections().keys().next().unwrap().clone();
    server.send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    update_times(10, &mut client, &mut server, config.interval);
}

