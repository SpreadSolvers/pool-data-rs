use alloy::sol;

sol! {
    #[sol(rpc)]
    interface UniswapV2Factory {
        function feeTo() view returns (address);
        function feeToSetter() view returns (address);
        function getPair(address tokenA, address tokenB) view returns (address pair);
        function allPairs(uint256) view returns (address pair);
        function allPairsLength() view returns (uint256);
        function createPair(address tokenA, address tokenB) returns (address pair);
        function setFeeTo(address) external;
        function setFeeToSetter(address) external;
    }
}
