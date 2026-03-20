use crate::{
    abi::uniswap_v4::{
        state_view::{PoolState, PoolStateView},
        ticks_getter::{EphemeralPoolTicksV4, IPositionManager, Tick},
    },
    provider::MyProvider,
    types::Protocol,
};
use alloy::{
    contract::Error as ContractError,
    primitives::{Address, B256, Bytes, address},
    sol_types::{SolType, sol_data::Array},
};
use log::debug;
use serde::Serialize;

const POSITION_MANAGER: Address = address!("bD216513d74C8cf14cf4747E6AaA6420FF64ee9e");

#[derive(Debug, Clone, Serialize)]
pub struct UniswapV4PoolData {
    pub ticks: Option<Vec<ProcessedTick>>,
    pub pool_id: B256,
    pub protocol: Protocol,
    pub creator_contract: Option<Address>,
    pub tokens: Vec<Address>,
    pub fee: u64,
    pub sqrt_price_x96: u128,
    pub liquidity: u128,
    pub tick: i64,
    pub tick_spacing: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedTick {
    pub index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
}

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
    pool_id: B256,
    position_manager: Address,
    provider: MyProvider,
    is_fetching_ticks: bool,
) -> Result<UniswapV4PoolData, Box<dyn std::error::Error>> {
    debug!("Position manager: {position_manager}");

    let position_manager_instance = IPositionManager::new(position_manager, provider.clone());

    let pool_manager: Address = position_manager_instance
        .poolManager()
        .call()
        .await
        .map_err(|e| format!("Failed to get pool manager from position manager: {e}"))?;

    let result: Result<Bytes, ContractError> =
        PoolStateView::deploy_builder(provider.clone(), pool_manager, pool_id)
            .call()
            .await;

    let pool_state = match &result {
        Ok(_) => return Err("Ephemeral StateView returned success (unexpected)".into()),
        Err(e) => {
            let bytes = revert_data_from_error(e)
                .ok_or_else(|| format!("Could not extract revert data: {e}"))?;
            PoolState::abi_decode(bytes.as_ref())
                .map_err(|e| format!("Failed to decode PoolState: {e}"))?
        }
    };

    let ticks = if is_fetching_ticks {
        Some(fetch_ticks(pool_id, position_manager, provider.clone()).await?)
    } else {
        None
    };

    let pool_id_prefix: [u8; 25] = pool_id[..25].try_into().expect("pool_id len");
    let pool_id_prefix = alloy::primitives::FixedBytes::from(pool_id_prefix);

    let pool_keys = position_manager_instance
        .poolKeys(pool_id_prefix)
        .call()
        .await
        .map_err(|e| format!("poolKeys failed: {e}"))?;

    let tick_spacing_abs = pool_keys
        .tickSpacing
        .try_into()
        .map(|s: i32| s.abs() as i64)
        .expect("tickSpacing to i64");

    let pool_data = UniswapV4PoolData {
        pool_id,
        protocol: Protocol::UniswapV4,
        creator_contract: Some(POSITION_MANAGER),
        tokens: vec![pool_keys.currency0, pool_keys.currency1],
        fee: pool_keys.fee.try_into().expect("fee to u64"),
        sqrt_price_x96: pool_state
            .sqrtPriceX96
            .try_into()
            .expect("sqrtPriceX96 to u128"),
        liquidity: pool_state.liquidity,
        tick: pool_state.tick.try_into().expect("tick to i64"),
        tick_spacing: tick_spacing_abs,
        ticks,
    };

    Ok(pool_data)
}

async fn fetch_ticks(
    pool_id: B256,
    position_manager: Address,
    provider: MyProvider,
) -> Result<Vec<ProcessedTick>, Box<dyn std::error::Error>> {
    let ticks_result: Result<Bytes, ContractError> =
        EphemeralPoolTicksV4::deploy_builder(provider.clone(), position_manager, pool_id)
            .call()
            .await;

    debug!("Ticks result: {:?}", ticks_result);

    let ticks = match &ticks_result {
        Ok(_) => return Err("EphemeralPoolTicksV4 returned success (unexpected)".into()),
        Err(e) => {
            let bytes = revert_data_from_error(e)
                .ok_or_else(|| format!("Could not extract ticks revert data: {e}"))?;

            let ticks_vec = Array::<Tick>::abi_decode(bytes.as_ref())
                .map_err(|e| format!("Failed to decode ticks: {e}"))?;

            ticks_vec
                .into_iter()
                .map(|t| {
                    let index = t
                        .index
                        .try_into()
                        .map_err(|_| "tick index out of i32 range".to_string())?;
                    Ok::<_, String>(ProcessedTick {
                        index,
                        liquidity_gross: t.liquidityGross,
                        liquidity_net: t.liquidityNet,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?
        }
    };

    Ok(ticks)
}
