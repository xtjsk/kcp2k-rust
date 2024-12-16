use bytes::Bytes;
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::{Callback, CallbackType};
use kcp2k_rust::kcp2k_channel::Kcp2KChannel;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
use kcp2k_rust::kcp2k_connection::Kcp2KConnection;
use std::process::exit;
use std::thread::sleep;

fn s_call_back(conn: &Kcp2KConnection, cb: Callback) {
    match cb.r#type {
        CallbackType::OnConnected => {
            println!("S - OnConnected {}", cb.conn_id);
            let _ = conn.send_data(Bytes::from(vec![1]), Kcp2KChannel::Reliable);
        }
        CallbackType::OnData => {
            println!(
                "S - received {:?} on channel {:?}",
                cb.data.as_ref(),
                cb.channel
            );
            let _ = conn.send_data(Bytes::from(vec![1]), Kcp2KChannel::Reliable);
        }
        CallbackType::OnDisconnected => {
            println!("OnDisconnected {}", cb.conn_id);
        }
        CallbackType::OnError => {
            println!("OnError {:?} {}", cb.conn_id, cb.error_message);
        }
    };
}

fn c_call_back(conn: &Kcp2KConnection, cb: Callback) {
     match cb.r#type {
        CallbackType::OnConnected => {
            println!("C - OnConnected {}", cb.conn_id);
            let _ = conn.send_data(Bytes::from(vec![0]), Kcp2KChannel::Reliable);
        }
        CallbackType::OnData => {
            println!(
                "C - received {:?} on channel {:?}",
                cb.data.as_ref(),
                cb.channel
            );
            let _ = conn.send_data(Bytes::from(vec![0]), Kcp2KChannel::Reliable);
        }
        CallbackType::OnDisconnected => {
            println!("OnDisconnected {}", cb.conn_id);
            exit(0);
        }
        CallbackType::OnError => {
            println!("OnError {:?} {}", cb.conn_id, cb.error_message);
        }
    };
}

fn main() {
    // 创建 KCP 服务器配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 服务器
    let server = Kcp2K::new_server(config, "0.0.0.0:3100".to_string(), s_call_back).unwrap();

    // 创建 KCP 客户端
    let client = Kcp2K::new_client(config, "127.0.0.1:3100".to_string(), c_call_back).unwrap();

    loop {
        // 服务器处理
        server.tick();
        // 客户端处理
        client.tick();
        sleep(std::time::Duration::from_millis(10));
    }
}
