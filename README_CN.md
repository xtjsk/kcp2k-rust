# kcp2k-rust

[English Document](README.md)

KCP2K（KCP with K2 network layer）协议的 Rust 实现，为游戏和实时应用提供可靠的 UDP 通信。

## 特性

- 可靠和不可靠的消息通道
- 服务器和客户端实现
- 可配置的 KCP 参数
- 基于事件的回调系统
- 线程安全通信
- 易用的 API

## 安装

在 `Cargo.toml` 中添加以下依赖：

```toml
[dependencies]
kcp2k_rust = { git = "https://github.com/xtjsk/kcp2k-rust.git" }
```


## 使用方法

### 服务器示例

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

### 客户端示例

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

## 配置

`Kcp2KConfig` 结构体允许你配置各种 KCP 参数：

```rust
let config = Kcp2KConfig {
    // 在这里添加你的自定义配置
    ..Default::default()
};
```

## 回调类型

库提供了几种回调类型：

- `OnConnected`: 建立连接时调用
- `OnDisconnected`: 连接终止时调用
- `OnData`: 收到数据时调用
- `OnError`: 发生错误时调用

## 通道

提供两种类型的通道：

- `Kcp2KChannel::Reliable`: 保证消息传递和顺序
- `Kcp2KChannel::Unreliable`: 快速传递，无保证

## 示例

查看 `examples` 目录获取完整的工作示例：

- `server.rs`: 基本的 KCP 服务器实现
- `client.rs`: 基本的 KCP 客户端实现
- `program.rs`: 展示各种特性的更复杂示例

## 许可证

本项目采用 MIT 许可证 - 详见 LICENSE 文件
