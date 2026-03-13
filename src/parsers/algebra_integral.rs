use crate::{
    abi::algebra_integral::pool::AlgebraIntegralPool, provider::MyProvider, types::Protocol,
};
use alloy::{primitives::Address, providers::Provider};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AlgebraIntegralPoolData {
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
    pub reserves: AlgebraIntegralPoolReserves,
    pub last_fee: u16,
    pub plugin: Address,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlgebraIntegralPoolReserves {
    pub reserve0: u128,
    pub reserve1: u128,
}

pub async fn fetch_pool_data(
    pool_id: Address,
    provider: MyProvider,
) -> Result<AlgebraIntegralPoolData, Box<dyn std::error::Error>> {
    let pool_contract = AlgebraIntegralPool::new(pool_id, provider.clone());

    // Pool immutables
    let multicall = provider
        .multicall()
        .add(pool_contract.factory())
        .add(pool_contract.token0())
        .add(pool_contract.token1())
        .add(pool_contract.fee())
        .add(pool_contract.tickSpacing())
        .add(pool_contract.maxLiquidityPerTick())
        .add(pool_contract.getReserves())
        .add(pool_contract.liquidity())
        .add(pool_contract.globalState())
        .add(pool_contract.plugin());

    let Ok((
        factory,
        token0,
        token1,
        fee,
        tick_spacing,
        max_liquidity_per_tick,
        reserves,
        liquidity,
        global_state,
        plugin,
    )) = multicall.aggregate().await
    else {
        return Err("Failed to get multicall result".into());
    };

    let tick_spacing = tick_spacing
        .abs()
        .try_into()
        .expect("Failed to convert tick spacing to i64");

    let pool_data = AlgebraIntegralPoolData {
        pool_address: pool_id,
        protocol: Protocol::AlgebraIntegral,
        creator_contract: Some(factory),
        tokens: vec![token0, token1],
        fee: fee.into(),
        sqrt_price_x96: global_state
            .price
            .try_into()
            .expect("Failed to convert price to u128"),
        liquidity,
        tick: global_state
            .tick
            .try_into()
            .expect("Failed to convert tick to i64"),
        tick_spacing,
        max_liquidity_per_tick,
        reserves: AlgebraIntegralPoolReserves {
            reserve0: reserves._0,
            reserve1: reserves._1,
        },
        last_fee: global_state.lastFee,
        plugin,
    };

    Ok(pool_data)
}
