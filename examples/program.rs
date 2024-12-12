use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::Callback;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use std::thread::sleep;

fn call_back(cb: Callback) {
    println!("{:?}", cb);
}

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let server = Kcp2K::new_server(config, "0.0.0.0:3100".to_string(), call_back).unwrap();

    // 创建 KCP 客户端
    let client = Kcp2K::new_client(config, "127.0.0.1:3100".to_string(), call_back).unwrap();

    loop {
        // 服务器处理
        server.tick();
        // 客户端处理
        client.tick();
        sleep(std::time::Duration::from_millis(10));
    }
}
