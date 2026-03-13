use crate::{
    abi::{
        ephemeral_pool_ticks_getter::EphemeralPoolTicksGetter::{self, Tick, Ticks},
        uniswap_v3::pool::UniswapV3Pool::{self},
    },
    provider::MyProvider,
    types::Protocol,
};
use alloy::sol_types::SolError;
use alloy::{
    contract::Error as ContractError,
    primitives::{Address, Bytes},
    providers::Provider,
};
use log::debug;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UniswapV3PoolData {
    pub pool_address: Address,
    pub protocol: Protocol,
    pub creator_contract: Option<Address>,
    pub tokens: Vec<Address>,
    pub fee: u64,
    pub sqrt_price_x96: u128,
    pub liquidity: u128,
    pub tick: i64,
    pub tick_spacing: i64,
    pub max_liquidity_per_tick: u128,
    pub ticks: Vec<Tick>,
}

/// Extracts revert data from contract error. Alloy's `as_revert_data()` only returns data when
/// `message.contains("revert")`; some RPCs (e.g. Fuse) return "VM execution error." so we also
/// parse the error payload's `data` field when it starts with "Reverted 0x".
fn revert_data_from_error(e: &ContractError) -> Option<Bytes> {
    if let Some(data) = e.as_revert_data() {
        return Some(data);
    }
    let ContractError::TransportError(te) = e else {
        return None;
    };
    let payload = te.as_error_resp()?;
    let raw = payload.data.as_ref()?;
    let s = raw.get().trim_matches('"').trim();
    let hex_str = s
        .strip_prefix("Reverted 0x")
        .or_else(|| s.strip_prefix("0x"))?;
    hex::decode(hex_str).ok().map(Bytes::from)
}

pub async fn fetch_pool_data(
    pool_id: Address,
    provider: MyProvider,
) -> Result<UniswapV3PoolData, Box<dyn std::error::Error>> {
    let result: Result<Bytes, ContractError> =
        EphemeralPoolTicksGetter::deploy_builder(provider.clone(), pool_id)
            .call()
            .await;

    let ticks = match &result {
        Ok(_) => {
            debug!("Ephemeral contract returned success (unexpected)");
            vec![]
        }
        Err(e) => {
            let decoded = e.as_decoded_error::<Ticks>();
            let ticks_vec = match decoded {
                Some(t) => {
                    debug!("Decoded Ticks from revert (as_decoded_error)");

                    t.ticks.clone()
                }
                None => revert_data_from_error(e)
                    .and_then(|bytes| Ticks::abi_decode(bytes.as_ref()).ok())
                    .map(|t| t.ticks)
                    .unwrap_or_else(|| {
                        debug!("Could not decode ticks from revert: {:?}", e);
                        vec![]
                    }),
            };
            ticks_vec
        }
    };

    debug!("Ticks: {:?}", ticks);

    let pool_contract = UniswapV3Pool::new(pool_id, provider.clone());

    // Pool immutables
    let multicall = provider
        .multicall()
        .add(pool_contract.factory())
        .add(pool_contract.token0())
        .add(pool_contract.token1())
        .add(pool_contract.fee())
        .add(pool_contract.tickSpacing())
        .add(pool_contract.maxLiquidityPerTick())
        .add(pool_contract.liquidity())
        .add(pool_contract.slot0());

    let Ok((factory, token0, token1, fee, tick_spacing, max_liquidity_per_tick, liquidity, slot0)) =
        multicall.aggregate().await
    else {
        return Err("Failed to get multicall result".into());
    };

    let tick_spacing = tick_spacing
        .abs()
        .try_into()
        .expect("Failed to convert tick spacing to i64");

    let pool_data = UniswapV3PoolData {
        pool_address: pool_id,
        protocol: Protocol::UniswapV3,
        creator_contract: Some(factory),
        tokens: vec![token0, token1],
        fee: fee.try_into().expect("Failed to convert fee to u64"),
        sqrt_price_x96: slot0
            .sqrtPriceX96
            .try_into()
            .expect("Failed to convert sqrt price to u128"),
        liquidity,
        tick: slot0
            .tick
            .try_into()
            .expect("Failed to convert tick to i64"),
        tick_spacing,
        max_liquidity_per_tick,
        ticks,
    };

    Ok(pool_data)
}
