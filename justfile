# build rust binary
alias b := build

build:
    cargo build --release

run rpc_url="http://localhost:8545" private_key="0x..." tokens_per_request="1000000000000000000" port="5556" host="127.0.0.1" gas_price_gwei="1" gas_limit="21000":
    RUST_LOG=info ./target/release/testnet-faucet \
        --rpc-url={{rpc_url}} \
        --private-key={{private_key}} \
        --tokens-per-request={{tokens_per_request}} \
        --port={{port}} \
        --host={{host}} \
        --gas-price-gwei={{gas_price_gwei}} \
        --gas-limit={{gas_limit}}
