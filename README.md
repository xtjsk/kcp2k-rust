# kcp2k-rust

[中文文档](README_CN.md)

A Rust implementation of KCP2K (KCP with a K2 network layer) protocol, providing reliable UDP communication for gaming and real-time applications.

## Features

- Reliable and unreliable message channels
- Server and client implementation
- Configurable KCP parameters
- Event-based callback system
- Thread-safe communication
- Easy-to-use API

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
kcp2k_rust = { git = "https://github.com/xtjsk/kcp2k-rust.git" }
```

## Usage

### Server Example

```rust
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::Callback;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;
fn call_back(cb: Callback) {
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

```

### Client Example

```rust
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_callback::Callback;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

fn call_back(cb: Callback) {
    println!("{:?}", cb);
}
fn main() {
    // 创建 KCP 客户端配置
    let config = Kcp2KConfig::default();

    // 创建 KCP 客户端
    let client = Kcp2K::new_client(config, "127.0.0.1:3100".to_string(), call_back).unwrap();

    loop {
        // 客户端处理
        client.tick();
    }
}
```

## Configuration

The `Kcp2KConfig` struct allows you to configure various KCP parameters:

```rust
let config = Kcp2KConfig {
    // Add your custom configuration here
    ..Default::default()
};
```

## Callback Types

The library provides several callback types:

- `OnConnected`: Called when a connection is established
- `OnDisconnected`: Called when a connection is terminated
- `OnData`: Called when data is received
- `OnError`: Called when an error occurs

## Channels

Two types of channels are available:

- `Kcp2KChannel::Reliable`: Guarantees message delivery and order
- `Kcp2KChannel::Unreliable`: Fast delivery without guarantees

## Examples

Check the `examples` directory for complete working examples:

- `server.rs`: A basic KCP server implementation
- `client.rs`: A basic KCP client implementation
- `program.rs`: A more complex example showing various features

## License

This project is licensed under the MIT License - see the LICENSE file for details.