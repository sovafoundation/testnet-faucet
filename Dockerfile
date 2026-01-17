# Use the official Rust image as the builder
FROM rust:1.81-slim-bookworm as builder

# Create a new empty shell project
WORKDIR /usr/src/app

# Install system dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY justfile ./

# Build the application
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/testnet-faucet /usr/local/bin/

# Create a non-root user
RUN useradd -m -u 1001 faucet
USER faucet

# Expose the default port
EXPOSE 5556

# Set the entrypoint
ENTRYPOINT ["testnet-faucet"]

# Default arguments that can be overridden
CMD ["--rpc-url", "http://localhost:8545", \
     "--private-key", "YOUR_PRIVATE_KEY_HERE", \
     "--tokens-per-request", "10000000000000000000", \
     "--port", "5556", \
     "--host", "0.0.0.0", \
     "--gas-price-gwei", "1", \
     "--gas-limit", "21000"]