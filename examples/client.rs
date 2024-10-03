use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_client::Client;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

fn main() {
    // 创建 KCP 客户端配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 客户端
    let (mut client, c_rx) = Client::new(config, "127.0.0.1:3100".to_string()).unwrap();


    loop {
        client.tick();
        if let Err(e) = client.send(vec![1, 2], Kcp2KChannel::Reliable) {
            println!("client send failed: {:?}", e);
            break;
        }
    }
}

