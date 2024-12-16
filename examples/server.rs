use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::Callback;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_connection::Kcp2KConnection;

fn call_back(_: &Kcp2KConnection, cb: Callback) {
    println!("{:?}", cb);
}
fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let server = Kcp2K::new_server(config, "0.0.0.0:3100".to_string(), call_back).unwrap();

    loop {
        // 服务器处理
        server.tick();
    }
}
