use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_client::Client;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_server::Server;

fn update_several_times(amount: usize, server: &mut Server, client: &mut Client) {
    for _ in 0..amount {
        server.tick();
        client.tick();
    }
}

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();


    // 创建 KCP 服务器
    let (mut server, s_rx) = Server::new(config, "0.0.0.0:3100".to_string()).unwrap();


    // 创建 KCP 客户端
    let (mut client,c_rx) = Client::new(config, "127.0.0.1:3100".to_string()).unwrap();

    // 服务器和客户端更新
    update_several_times(5, &mut server, &mut client);


    client.send(vec![1, 2], Kcp2KChannel::Reliable).unwrap();
    update_several_times(10, &mut server, &mut client);

    let id = server.get_connections().keys().next().unwrap().clone();
    server.send(id, vec![3, 4], Kcp2KChannel::Unreliable).unwrap();
    update_several_times(10, &mut server, &mut client);
}

