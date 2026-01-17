# Testnet Faucet Service

A lightweight HTTP service that distributes testnet tokens from a pre-funded wallet. Built with Rust using actix-web and alloy for Ethereum interactions.

## Overview

This service provides a simple HTTP endpoint that allows users to request testnet tokens. It uses a pre-funded wallet to send tokens to requested addresses, making it perfect for testnet and development environments.

The address requesting funds must have a balance of zero else it will be denied the request.

## Prerequisites

- Rust 1.81 or later
- A network RPC endpoint
- A funded wallet private key

## Installation

1. Clone the repository:
```bash
git clone https://github.com/sovafoundation/testnet-faucet.git
cd testnet-faucet
```

2. Build the project:
```bash
cargo build --release

# or using justfile

just b
```

## Configuration
The service is configured via command-line arguments:
```bash
RUST_LOG=info ./target/release/testnet-faucet \
  --rpc-url <RPC_URL> \
  --private-key <PRIVATE_KEY> \
  --tokens-per-request <AMOUNT> \
  --port <PORT> \
  --host <HOST> \
  --gas-price-gwei <GAS_PRICE> \
  --gas-limit <GAS_LIMIT>

  # or using justfile
  just run rpc_url=http://localhost:8545 private_key=0x... tokens_per_request=1000000000000000000 port=5556 host=127.0.0.1 gas_price_gwei=1 gas_limit=21000
```
### Arguments
| Argument | Description | Default |
| --- | --- | --- |
| `--rpc-url` | Network RPC endpoint | `http://localhost:8545` |
| `--private-key` | Faucet wallet private key (without 0x prefix) | *Required* |
| `--tokens-per-request` | Amount of tokens to send per request (in wei) | `1000000000000000000` (1e18) |
| `--port` | Server port to listen on | `5556` |
| `--host` | Server host to bind to | `127.0.0.1` |
| `--gas-price-gwei` | Gas price in gwei | `1` |
| `--gas-limit` | Gas limit for transactions | `21000` |

## Using Docker
```
# Build the image
docker build -t testnet-faucet .

# Run the container (replace with your actual values)
docker run -p 5556:5556 -d --name testnet-faucet \
  --network "YOUR_NETWORK_NAME" \
	--name testnet-faucet \
  testnet-faucet \
  --rpc-url "YOUR_RPC_URL" \
  --private-key "YOUR_PRIVATE_KEY" \
  --host "0.0.0.0" \
  --tokens-per-request 10000000000000000000
```

## API Endpoints
### Request Tokens
```bash
curl -X POST http://localhost:5556/faucet \
  -H "Content-Type: application/json" \
  -d '{"address": "0x0314cF79B4D9aC9192d5768690ACf15C24a940ad"}'
```

### Health Check
```bash
curl -X GET http://localhost:5556/health \
  -H "Content-Type: application/json"
```

## License
This project is licensed under the MIT License.


