# CosmWasm Client RS

A Rust client library for interacting with CosmWasm-enabled blockchain networks. This library provides a simple and efficient way to perform operations like contract deployment, message execution, and event listening on CosmWasm chains.

## Features

- **Contract Operations**: Deploy, instantiate and execute CosmWasm smart contracts
- **Event Listening**: Subscribe to and process blockchain events
- **Wallet Management**: Handle blockchain accounts and transactions
- **Chain Interaction**: Communicate with the blockchain through gRPC and WebSocket
- **Type-safe Interface**: Leverage Rust's type system for safe contract interactions

## Prerequisites

- Rust 2021 edition or later
- A running CosmWasm-compatible blockchain node
- Access to both gRPC and WebSocket endpoints of the blockchain node

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cosmwasm-client-rs = "0.1.0"
```

## Usage

Here's a basic example of how to use the client:

```rust
use cosmwasm_client_rs::CosmWasmClient;
use cosmrs::AccountId;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize client
    let grpc_url = "http://localhost:9090";
    let ws_url = "ws://localhost:26657/websocket";
    let private_key = "your_private_key";
    let contract = AccountId::from_str("your_contract_address")?;

    let client = CosmWasmClient::new(grpc_url, ws_url, private_key, Some(contract))
        .await?;

    // Perform operations
    // ... your code here ...

    Ok(())
}
```

Check the `examples` directory for more detailed usage examples.

## Examples

The repository includes several examples demonstrating different features:

- `contract_operations.rs`: Shows how to perform contract operations
- `event_listener.rs`: Demonstrates event subscription and handling

## Dependencies

Key dependencies include:
- `cosmos-sdk-proto`: For blockchain communication
- `tendermint-rpc`: For RPC interactions
- `cosmrs`: For CosmWasm operations
- `tokio`: For async runtime
- `tonic`: For gRPC support

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Disclaimer

This software is in active development. Please use it carefully in production environments.
