use std::str::FromStr;
use std::time::Instant;

use alloy::{primitives::Address, transports::http::reqwest::Url};
use clap::error::{ContextKind, ContextValue};
use clap::{Parser, ValueHint};
use log::{debug, warn};

use pool_data_rs::parsers::{algebra_integral, uniswap_v2, uniswap_v3, uniswap_v4};
use pool_data_rs::provider::create_provider;
use pool_data_rs::types::Protocol;

#[derive(Debug, Parser)]
#[command(name = "pool")]
#[command(about = "Pool data retrieval CLI")]
struct Args {
    pool_id: String,
    #[arg(value_enum)]
    protocol: Protocol,
    #[arg(value_hint = ValueHint::Url)]
    rpc_url: String,
}

async fn run(args: Args) -> Result<(), clap::Error> {
    let time = Instant::now();

    debug!("Args: {args:?}");

    let pool_id = if args.protocol == Protocol::UniswapV4 {
        Address::ZERO // Not used for V4
    } else {
        Address::from_str(&args.pool_id).map_err(|_| {
            clap::Error::new(clap::error::ErrorKind::InvalidValue)
        })?
    };

    debug!("Pool ID: {}", args.pool_id);

    let Ok(rpc_url) = Url::parse(&args.rpc_url) else {
        warn!("Invalid RPC URL: {}", args.rpc_url);

        let mut err = clap::Error::new(clap::error::ErrorKind::InvalidValue);

        err.insert(
            ContextKind::InvalidValue,
            ContextValue::String(format!("Invalid RPC URL: {}", args.rpc_url.clone())),
        );

        return Err(err);
    };

    debug!("Protocol: {:?}", args.protocol);

    debug!("RPC URL: {rpc_url:?}");

    debug!(
        "Getting pool data for pool_id: {} and rpc_url: {}",
        args.pool_id, args.rpc_url
    );

    let provider = create_provider(&args.rpc_url).await.map_err(|e| {
        warn!("Failed to create provider: {e}");

        let mut err = clap::Error::new(clap::error::ErrorKind::Io);

        err.insert(
            ContextKind::Custom,
            ContextValue::String("Failed to create Web3 provider".to_string()),
        );

        err.insert(ContextKind::Custom, ContextValue::String(e.to_string()));

        err
    })?;

    match args.protocol {
        Protocol::UniswapV2 => {
            let pool_data = uniswap_v2::fetch_pool_data(pool_id, provider.clone())
                .await
                .map_err(|e| {
                    warn!("Failed to parse pool data: {e}");
                    clap::Error::new(clap::error::ErrorKind::Io)
                })?;

            println!(
                "{}",
                serde_json::to_string_pretty(&pool_data).expect("Failed to serialize pool data")
            );
        }
        Protocol::UniswapV3 => {
            let pool_data = uniswap_v3::fetch_pool_data(pool_id, provider.clone())
                .await
                .map_err(|e| {
                    warn!("Failed to parse pool data: {e}");
                    clap::Error::new(clap::error::ErrorKind::Io)
                })?;

            println!(
                "{}",
                serde_json::to_string_pretty(&pool_data).expect("Failed to serialize pool data")
            );
        }
        Protocol::UniswapV4 => {
            let pool_id_b256 = alloy::primitives::B256::from_str(&args.pool_id).map_err(|_| {
                let mut err = clap::Error::new(clap::error::ErrorKind::InvalidValue);
                err.insert(
                    ContextKind::InvalidValue,
                    ContextValue::String("Pool ID must be 32-byte hex (0x + 64 chars) for Uniswap V4".to_string()),
                );
                err
            })?;

            let pool_data = uniswap_v4::fetch_pool_data(pool_id_b256, provider.clone())
                .await
                .map_err(|e| {
                    warn!("Failed to parse pool data: {e}");
                    clap::Error::new(clap::error::ErrorKind::Io)
                })?;

            println!(
                "{}",
                serde_json::to_string_pretty(&pool_data).expect("Failed to serialize pool data")
            );
        }
        Protocol::AlgebraIntegral => {
            let pool_data = algebra_integral::fetch_pool_data(pool_id, provider.clone())
                .await
                .map_err(|e| {
                    warn!("Failed to parse pool data: {e}");
                    clap::Error::new(clap::error::ErrorKind::Io)
                })?;
            println!(
                "{}",
                serde_json::to_string_pretty(&pool_data).expect("Failed to serialize pool data")
            );
        }
    }

    debug!("Time taken: {:?}", time.elapsed());

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();

    dotenv::dotenv().ok();

    let args = Args::parse();
    if let Err(e) = run(args).await {
        // debug!("{:?}", e);

        eprintln!("CLI failed with the following errors:");

        for context in e.context() {
            eprintln!(
                "Reason: {:?}, Details: {}",
                context.0.to_string(),
                context.1.to_string()
            );
        }
    }
}
