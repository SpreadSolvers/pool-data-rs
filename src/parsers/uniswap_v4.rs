use crate::{
    abi::uniswap_v4::state_view::{IPositionManagerPoolKeys, IStateView},
    provider::MyProvider,
    types::Protocol,
};
use alloy::{
    primitives::{Address, B256, address},
    providers::Provider,
};
use log::debug;
use serde::Serialize;

const STATE_VIEW: Address = address!("7fFE42C4a5DEeA5b0feC41C94C136Cf115597227");
const POSITION_MANAGER: Address = address!("bD216513d74C8cf14cf4747E6AaA6420FF64ee9e");
const MIN_TICK: i32 = -887272;
const MAX_TICK: i32 = 887272;

#[derive(Debug, Clone, Serialize)]
pub struct UniswapV4PoolData {
    pub ticks: Vec<ProcessedTick>,
    pub pool_id: B256,
    pub protocol: Protocol,
    pub creator_contract: Option<Address>,
    pub tokens: Vec<Address>,
    pub fee: u64,
    pub sqrt_price_x96: u128,
    pub liquidity: u128,
    pub tick: i64,
    pub tick_spacing: i64,
    pub max_liquidity_per_tick: u128,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedTick {
    pub index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
}

fn compress_tick(tick: i32, tick_spacing: i32) -> i32 {
    let mut compressed = tick / tick_spacing;
    if tick < 0 && tick % tick_spacing != 0 {
        compressed -= 1;
    }
    compressed
}

fn get_word_positions(tick_spacing: i32) -> (i16, i16) {
    let compressed_lower = compress_tick(MIN_TICK, tick_spacing);
    let compressed_upper = compress_tick(MAX_TICK, tick_spacing);
    (
        (compressed_lower >> 8) as i16,
        (compressed_upper >> 8) as i16,
    )
}

pub async fn fetch_pool_data(
    pool_id: B256,
    provider: MyProvider,
) -> Result<UniswapV4PoolData, Box<dyn std::error::Error>> {
    let state_view = IStateView::new(STATE_VIEW, provider.clone());
    let position_manager = IPositionManagerPoolKeys::new(POSITION_MANAGER, provider.clone());

    // bytes25 = first 25 bytes of pool_id
    let pool_id_prefix: [u8; 25] = pool_id[..25].try_into().expect("pool_id len");
    let pool_id_prefix = alloy::primitives::FixedBytes::from(pool_id_prefix);

    let pool_keys = position_manager
        .poolKeys(pool_id_prefix)
        .call()
        .await
        .map_err(|e| format!("poolKeys failed: {e}"))?;

    let currency0 = pool_keys.currency0;
    let currency1 = pool_keys.currency1;
    let fee = pool_keys.fee;
    let tick_spacing = pool_keys.tickSpacing;

    let tick_spacing_i32: i32 = tick_spacing.try_into().expect("tickSpacing to i32");
    let tick_spacing_abs = tick_spacing_i32.abs() as i64;

    let multicall = provider
        .multicall()
        .add(state_view.getSlot0(pool_id))
        .add(state_view.getLiquidity(pool_id));

    let (slot0, liquidity) = multicall
        .aggregate()
        .await
        .map_err(|e| format!("StateView multicall failed: {e}"))?;

    let (word_pos_lower, word_pos_upper) = get_word_positions(tick_spacing_i32);

    let mut ticks = Vec::new();
    for word in word_pos_lower..=word_pos_upper {
        let bitmap = state_view
            .getTickBitmap(pool_id, word)
            .call()
            .await
            .unwrap_or_default();

        let bitmap_u128: u128 = bitmap.to::<u128>();
        for bit in 0..256u32 {
            if bitmap_u128 & (1u128 << bit) == 0 {
                continue;
            }
            let tick = ((word as i32) << 8 | bit as i32) * tick_spacing_i32;
            let tick_i24: alloy::primitives::Signed<24, 1> =
                tick.try_into().expect("tick to int24");
            let tick_result = state_view.getTickLiquidity(pool_id, tick_i24).call().await;

            let (liquidity_gross, liquidity_net) = match tick_result {
                Ok(r) => (r.liquidityGross, r.liquidityNet),
                Err(_) => (0u128, 0i128),
            };

            ticks.push(ProcessedTick {
                index: tick,
                liquidity_gross,
                liquidity_net,
            });
        }
    }

    debug!("Ticks: {:?}", ticks);

    let pool_data = UniswapV4PoolData {
        pool_id,
        protocol: Protocol::UniswapV4,
        creator_contract: Some(POSITION_MANAGER),
        tokens: vec![currency0, currency1],
        fee: fee.try_into().expect("fee to u64"),
        sqrt_price_x96: slot0.sqrtPriceX96.try_into().expect("sqrtPriceX96 to u128"),
        liquidity,
        tick: slot0.tick.try_into().expect("tick to i64"),
        tick_spacing: tick_spacing_abs,
        max_liquidity_per_tick: 0, // V4 doesn't expose this per-pool
        ticks,
    };

    Ok(pool_data)
}
