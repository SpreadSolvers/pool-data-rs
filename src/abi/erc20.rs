use alloy::sol;

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function name() view returns (string memory);
        function symbol() view returns (string memory);
        function decimals() view returns (uint8);
    }
}
