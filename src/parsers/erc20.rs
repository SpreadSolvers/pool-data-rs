use std::fmt::Display;

use alloy::{primitives::Address, providers::Provider, rpc::types::TransactionRequest};

use crate::{
    abi::erc20::IERC20::{self, decimalsCall, nameCall, symbolCall},
    provider::MyProvider,
};

#[derive(Debug, Clone)]
pub struct ERC20Metadata {
    name: String,
    symbol: String,
    decimals: u8,
}

impl Display for ERC20Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ERC20Metadata {{ name: {}, symbol: {}, decimals: {} }}",
            self.name, self.symbol, self.decimals
        )
    }
}

pub async fn fetch_erc20_metadata(
    address: Address,
    provider: MyProvider,
) -> Result<ERC20Metadata, Box<dyn std::error::Error>> {
    let erc20_contract = IERC20::new(address, provider.clone());

    let name_call = erc20_contract.name().into_transaction_request();
    let symbol_call = erc20_contract.symbol().into_transaction_request();
    let decimals_call = erc20_contract.decimals().into_transaction_request();

    let (name_result, symbol_result, decimals_result) = tokio::try_join!(
        provider.call(name_call).decode_resp::<nameCall>(),
        provider.call(symbol_call).decode_resp::<symbolCall>(),
        provider.call(decimals_call).decode_resp::<decimalsCall>(),
    )?;

    let Ok(name) = name_result else {
        return Err("Failed to to fetch and decode name call".into());
    };

    let Ok(symbol) = symbol_result else {
        return Err("Failed to to fetch and decode symbol call".into());
    };

    let Ok(decimals) = decimals_result else {
        return Err("Failed to to fetch and decode decimals call".into());
    };

    Ok(ERC20Metadata {
        name,
        symbol,
        decimals,
    })
}
