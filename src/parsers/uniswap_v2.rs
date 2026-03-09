use crate::{
    abi::uniswap_v2::{
        factory::UniswapV2Factory,
        pool::UniswapV2Pool::{self, factoryCall, getReservesCall, token0Call, token1Call},
    },
    provider::MyProvider,
    types::{PoolData, Protocol},
};
use alloy::{
    primitives::{Address, Uint},
    providers::Provider,
    rpc,
    transports::http::reqwest::Url,
};
use log::debug;

#[derive(Debug, Clone)]
pub struct UniswapV2PoolData {
    pub pool_address: Address,
    pub protocol: Protocol,
    pub creator_contract: Option<Address>,
    pub tokens: Vec<Address>,
    pub fee: u64,
}

impl PoolData for UniswapV2PoolData {
    fn pool_id(&self) -> String {
        self.pool_address.to_string()
    }

    fn protocol(&self) -> Protocol {
        self.protocol.clone()
    }

    fn creator_contract(&self) -> Option<&Address> {
        self.creator_contract.as_ref()
    }

    fn tokens(&self) -> Vec<&Address> {
        self.tokens.iter().collect()
    }

    fn fee(&self) -> u64 {
        self.fee
    }
}

pub async fn fetch_pool_data(
    pool_id: Address,
    provider: MyProvider,
) -> Result<UniswapV2PoolData, Box<dyn std::error::Error>> {
    let pool_contract = UniswapV2Pool::new(pool_id, provider.clone());

    let factory_call = pool_contract.factory().into_transaction_request();
    let token0_call = pool_contract.token0().into_transaction_request();
    let token1_call = pool_contract.token1().into_transaction_request();
    let get_reserves_call = pool_contract.getReserves().into_transaction_request();

    let (factory, token0, token1, get_reserves) = tokio::try_join!(
        provider.call(factory_call).decode_resp::<factoryCall>(),
        provider.call(token0_call).decode_resp::<token0Call>(),
        provider.call(token1_call).decode_resp::<token1Call>(),
        provider
            .call(get_reserves_call)
            .decode_resp::<getReservesCall>(),
    )?;

    let Ok(factory) = factory else {
        return Err("Failed to to fetch and decode factory call".into());
    };

    let Ok(token0) = token0 else {
        return Err("Failed to to fetch and decode token0 call".into());
    };

    let Ok(token1) = token1 else {
        return Err("Failed to to fetch and decode token1 call".into());
    };

    let Ok(get_reserves) = get_reserves else {
        return Err("Failed to to fetch and decode get reserves call".into());
    };

    let (reserve0, reserve1, block_timestamp_last) = get_reserves.into();

    // let fee = fetch_fee_from_factory(factory, provider.clone()).await?;

    debug!("Factory call: {factory:?}");
    debug!("Token0 call: {token0:?}");
    debug!("Token1 call: {token1:?}");
    debug!("Get reserves call: {reserve0:?}, {reserve1:?}, {block_timestamp_last:?}");

    let pool_data = UniswapV2PoolData {
        pool_address: pool_id,
        protocol: Protocol::UniswapV2,
        creator_contract: Some(factory),
        tokens: vec![token0, token1],
        fee: 30,
    };

    Ok(pool_data)
}
