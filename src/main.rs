use std::{io::Result, sync::Arc};

use clap::Parser;

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};

use alloy_primitives::{Address, U256};
use alloy_provider::{
    network::{EthereumWallet, TransactionBuilder},
    Provider, ProviderBuilder, RootProvider,
};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_transport_http::{Client, Http};

use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// RPC URL for the L2 network
    #[arg(long, default_value = "http://localhost:8545")]
    rpc_url: String,

    /// Private key for the faucet wallet (without 0x prefix)
    #[arg(long)]
    private_key: String,

    /// Amount of tokens to send per request (in wei)
    #[arg(long, default_value = "1000000000000000000")]
    tokens_per_request: String,

    /// Server port to listen on
    #[arg(long, default_value = "5556")]
    port: u16,

    /// Server host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Gas price in gwei
    #[arg(long, default_value = "1")]
    gas_price_gwei: u64,

    /// Gas limit for transactions
    #[arg(long, default_value = "21000")]
    gas_limit: u64,
}

// Request and Response structures
#[derive(Deserialize)]
struct FaucetRequest {
    address: String,
}

#[derive(Serialize)]
struct FaucetResponse {
    transaction_hash: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// App state structure
struct AppState {
    provider: Arc<RootProvider<Http<Client>>>,
    wallet: EthereumWallet,
    tokens_per_request: U256,
    gas_price: U256,
    gas_limit: U256,
}

/// Balance of the address receiving tokens must be zero. Balance of the sender must be greater than the tokens requested.
async fn send_tokens(data: web::Json<FaucetRequest>, state: web::Data<AppState>) -> impl Responder {
    let to_address = match Address::parse_checksummed(&data.address, None) {
        Ok(addr) => addr,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid address".to_string(),
            })
        }
    };

    // Get the wallet address from state
    let from_address = state.wallet.default_signer().address();

    // Balance validations
    let sender_balance = match state.provider.get_balance(from_address).await {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get balance: {}", e),
            })
        }
    };
    if sender_balance < state.tokens_per_request {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Insufficient balance".to_string(),
        });
    }

    let receiver_balance = match state.provider.get_balance(to_address).await {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get balance: {}", e),
            })
        }
    };
    if receiver_balance > U256::ZERO {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Receiver already has a balance greater than 0".to_string(),
        });
    }

    // Get the next nonce for the wallet
    let nonce = match state.provider.get_transaction_count(from_address).await {
        Ok(n) => n,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get nonce: {}", e),
            })
        }
    };

    // Get the current chain id
    let chain_id = match state.provider.get_chain_id().await {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get chain ID: {}", e),
            })
        }
    };

    // Build the transaction request
    let mut tx = TransactionRequest::default()
        .to(to_address)
        .nonce(nonce)
        .value(state.tokens_per_request)
        .gas_limit(state.gas_limit.to::<u64>())
        .max_fee_per_gas(state.gas_price.to::<u128>())
        .max_priority_fee_per_gas(state.gas_price.to::<u128>());

    tx.set_chain_id(chain_id);

    // Build and sign the transaction
    let tx_envelope = match tx.build(&state.wallet).await {
        Ok(envelope) => envelope,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to build transaction: {}", e),
            })
        }
    };

    // Send the transaction
    match state.provider.send_tx_envelope(tx_envelope).await {
        Ok(receipt) => {
            info!(
                "sent tokens: {:?} to {:?}. Tx hash: {:?}",
                state.tokens_per_request,
                to_address,
                receipt.tx_hash()
            );
            HttpResponse::Ok().json(FaucetResponse {
                transaction_hash: format!("{:?}", receipt.tx_hash()),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to send transaction: {}", e),
        }),
    }
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[actix_web::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // strip 0x prefix from private key if present
    let pk = args.private_key.strip_prefix("0x").unwrap_or(&args.private_key);

    // Setup wallet
    let private_key_bytes = hex::decode(pk)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
    let private_key_bytes: [u8; 32] = private_key_bytes.try_into().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid private key length",
        )
    })?;
    let fixed_bytes = alloy_primitives::FixedBytes::from(private_key_bytes);
    let signer = PrivateKeySigner::from_bytes(&fixed_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
    let wallet = EthereumWallet::from(signer);

    // Setup provider with wallet
    let url = args.rpc_url.parse().expect("should parse rpc url");
    let provider = ProviderBuilder::new().on_http(url);

    // Parse tokens per request
    let tokens_per_request = U256::from_str_radix(&args.tokens_per_request, 10)
        .expect("Invalid tokens_per_request value");

    // Convert gas price from gwei to wei
    let gas_price = U256::from(args.gas_price_gwei) * U256::from(1_000_000_000);
    let gas_limit = U256::from(args.gas_limit);

    // Create app state
    let state = web::Data::new(AppState {
        provider: Arc::new(provider),
        wallet,
        tokens_per_request,
        gas_price,
        gas_limit,
    });

    // Start server
    HttpServer::new(move || {
        // Setup CORS
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(state.clone())
            .route("/faucet", web::post().to(send_tokens))
            .route("/health", web::get().to(health_check))
    })
    .bind((args.host, args.port))?
    .run()
    .await?;

    Ok(())
}
