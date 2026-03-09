use alloy::primitives::Address;
use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum Protocol {
    #[value(name = "uni-v2")]
    UniswapV2,
    #[value(name = "uni-v3")]
    UniswapV3,
    #[value(name = "uni-v4")]
    UniswapV4,
}

pub trait PoolData: Sized + 'static {
    fn pool_id(&self) -> String;
    fn protocol(&self) -> Protocol;
    fn creator_contract(&self) -> Option<&Address>;
    fn tokens(&self) -> Vec<&Address>;
    fn fee(&self) -> u64;
}
