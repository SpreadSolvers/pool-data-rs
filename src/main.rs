use std::str::FromStr;
use std::time::Instant;

use alloy::{primitives::Address, transports::http::reqwest::Url};
use clap::{Parser, ValueHint};
use log::{debug, warn};

use pool_data_rs::parsers::{self, uniswap_v2};
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

    let Ok(pool_id) = Address::from_str(&args.pool_id) else {
        return Err(clap::Error::new(clap::error::ErrorKind::InvalidValue));
    };

    debug!("Pool ID: {pool_id:?}");

    let Ok(rpc_url) = Url::parse(&args.rpc_url) else {
        return Err(clap::Error::new(clap::error::ErrorKind::InvalidValue));
    };

    debug!("Protocol: {:?}", args.protocol);

    debug!("RPC URL: {rpc_url:?}");

    debug!(
        "Getting pool data for pool_id: {} and rpc_url: {}",
        args.pool_id, args.rpc_url
    );

    let provider = create_provider(&args.rpc_url).await.map_err(|e| {
        warn!("Failed to create provider: {e}");
        clap::Error::new(clap::error::ErrorKind::Io)
    })?;

    if args.protocol != Protocol::UniswapV2 {
        return Err(clap::Error::new(clap::error::ErrorKind::InvalidValue));
    }

    let pool_data = uniswap_v2::fetch_pool_data(pool_id, provider.clone())
        .await
        .map_err(|e| {
            warn!("Failed to parse pool data: {e}");
            clap::Error::new(clap::error::ErrorKind::Io)
        })?;

    debug!("Pool data: {:?}", pool_data);

    println!("Pool data: {}\n", pool_data);

    for (i, token) in pool_data.tokens.iter().enumerate() {
        let metadata = parsers::erc20::fetch_erc20_metadata(token.clone(), provider.clone())
            .await
            .map_err(|e| {
                warn!("Failed to parse erc20 metadata: {e}");
                clap::Error::new(clap::error::ErrorKind::Io)
            })?;
        debug!("Token metadata: {}", metadata);
        println!("Token{i} metadata: {}\n", metadata);
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
        eprintln!("{e}");
        std::process::exit(1);
    }
}
