use alloy::sol;

sol! {
    #[sol(rpc)]
    interface UniswapV2Pool {
        function token0() view returns (address);
        function token1() view returns (address);
        function getReserves() view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function factory() view returns (address);
    }
}
