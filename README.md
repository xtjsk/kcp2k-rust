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

## Dependencies

- kcp = "0.5.3"
- bytes = "1.7.1"
- rand = "0.9.0-alpha.2"
- socket2 = "0.5.7"
- tklog = "0.2.1"
- dashmap = "6.1.0"
- crossbeam-channel = "0.5.13"

## Usage

### Server Example

```rust
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

// Create server configuration
let config = Kcp2KConfig::default();

// Create KCP server
let (server, s_rx) = Kcp2K::new_server(config, "0.0.0.0:3100".to_string()).unwrap();

loop {
    // Server tick
    server.tick();
    
    // Handle callbacks
    if let Ok(cb) = s_rx.try_recv() {
        match cb.callback_type {
            CallbackType::OnConnected => {
                println!("Client connected: {}", cb.connection_id);
            }
            CallbackType::OnData => {
                println!("Received data on channel: {:?}", cb.channel);
            }
            // ... handle other callbacks
        }
    }
}
```

### Client Example

```rust
use kcp2k_rust::kcp2k::Kcp2K;
use kcp2k_rust::kcp2k_config::Kcp2KConfig;

// Create client configuration
let config = Kcp2KConfig::default();

// Create KCP client
let (client, c_rx) = Kcp2K::new_client(config, "127.0.0.1:7777".to_string()).unwrap();

loop {
    // Client tick
    client.tick();
    
    // Handle callbacks
    if let Ok(cb) = c_rx.try_recv() {
        match cb.callback_type {
            CallbackType::OnConnected => {
                println!("Connected to server");
            }
            CallbackType::OnData => {
                println!("Received data on channel: {:?}", cb.channel);
            }
            // ... handle other callbacks
        }
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