use alloy::sol;

sol! {
    struct Tick {
        int24 index;
        uint128 liquidityGross;
        int128 liquidityNet;
    }

    /// @notice Minimal interface for PositionManager.poolKeys
    interface IPositionManagerPoolKeys {
        function poolKeys(bytes25 poolId)
            external
            view
            returns (address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks);
    }

    /// @notice Minimal interface for StateView tick queries (PoolId = bytes32)
    interface IStateViewTicks {
        function getTickBitmap(bytes32 poolId, int16 tick) external view returns (uint256 tickBitmap);
        function getTickLiquidity(bytes32 poolId, int24 tick)
            external
            view
            returns (uint128 liquidityGross, int128 liquidityNet);
    }

    /// @notice A lens that fetches all populated ticks for a Uniswap V4 pool without deployment
    /// @author Aperture Finance
    /// @dev Uses PositionManager + poolId to resolve poolKey, StateView for tick data. Return data via revert Ticks(ticks).
    #[sol(rpc, bytecode="60808060405260608161057c803803809161001a82856104e3565b8339810103126102f75761002d81610506565b90602460a0604061004060208501610506565b93015193604051928380926386b6be7d60e01b825266ffffffffffffff1988166004830152600180861b03165afa908115610303575f9161046e575b5060020b5f81620d89e719071281620d89e719050360020b60081d60010b905f81620d89e8071281620d89e8050360020b60081d60010b915f94819460018060a01b0316945b848113156103b557506100d48661053c565b956100e260405197886104e3565b8087526100f1601f199161053c565b015f5b8181106103665750505f915b8481131561018c578660405160208101918160408101916020855280518093526020606083019101925f5b81811061014b575050610147925003601f1981018352826104e3565b5190fd5b8451805160020b84526020808201516001600160801b031681860152604091820151600f0b918501919091529094019385935060609092019160010161012b565b60405163071f32d360e21b815260048101839052600182900b60248201526020816044818a5afa908115610303575f91610335575b50801561032b575f905b61010082106101e45750506101df9061051a565b610100565b90936001851b821615610322578260081b858101905f878312911290801582169115161761030e57869060020b028060020b90810361030e576040516332bb6ad560e21b81528560048201528160248201526040816044818d5afa908115610303578b915f915f916102a1575b509161029893918593604061027686602061026e60019b88610553565b510195610553565b510190600f0b9052858060801b03169052610291828d610553565b515261052e565b945b01906101cb565b94925050506040833d82116102fb575b816102be604093836104e3565b810103126102f7578251926001600160801b03841684036102f757602001519182600f0b83036102f757909290918b9190610298610251565b5f80fd5b3d91506102b1565b6040513d5f823e3d90fd5b634e487b7160e01b5f52601160045260245ffd5b9360019061029a565b506101df9061051a565b90506020813d821161035e575b8161034f602093836104e3565b810103126102f757515f6101c1565b3d9150610342565b6040516060810191906001600160401b038311818410176103a1576020926040525f81525f838201525f604082015282828b010152016100f4565b634e487b7160e01b5f52604160045260245ffd5b60405163071f32d360e21b815260048101839052600182900b60248201526020816044818a5afa908115610303575f9161043d575b508015610433575f5b610100811061040c5750506104079061051a565b6100c2565b6001811b821661041f575b6001016103f3565b9761042b60019161052e565b989050610417565b506104079061051a565b90506020813d8211610466575b81610457602093836104e3565b810103126102f757515f6103ea565b3d915061044a565b905060a0813d60a0116104db575b8161048960a093836104e3565b810103126102f75761049a81610506565b506104a760208201610506565b50604081015162ffffff8116036102f7576060810151908160020b82036102f75760806104d49101610506565b505f61007c565b3d915061047c565b601f909101601f19168101906001600160401b038211908210176103a157604052565b51906001600160a01b03821682036102f757565b6001600160ff1b03811461030e5760010190565b5f19811461030e5760010190565b6001600160401b0381116103a15760051b60200190565b80518210156105675760209160051b010190565b634e487b7160e01b5f52603260045260245ffdfe")]
    contract EphemeralPoolTicksV4 {
        int24 internal constant MIN_TICK = -887272;
        int24 internal constant MAX_TICK = -MIN_TICK;

        /// @param positionManager PositionManager address (e.g. 0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e on Ethereum)
        /// @param stateView StateView helper address (e.g. 0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227 on Ethereum)
        /// @param poolId Pool ID (bytes32). Use bytes25(poolId) to get poolKey from PositionManager.poolKeys
        constructor(address positionManager, address stateView, bytes32 poolId) payable {
            Tick[] memory ticks = getAllTicks(positionManager, stateView, poolId);
            bytes memory returnData = abi.encode(ticks);
            assembly ("memory-safe") {
                revert(add(returnData, 0x20), mload(returnData))
            }
        }

        /// @notice Get all populated ticks for a V4 pool
        /// @param positionManager PositionManager address
        /// @param stateView StateView helper address
        /// @param poolId The pool ID (bytes32)
        function getAllTicks(address positionManager, address stateView, bytes32 poolId)
            public
            view
            returns (Tick[] memory ticks)
        {
            // bytes25 = poolId with last 7 bytes stripped
            bytes25 poolIdPrefix = bytes25(poolId);
            int24 tickSpacing = _getTickSpacing(positionManager, poolIdPrefix);

            (int16 wordPosLower, int16 wordPosUpper) = _getWordPositions(tickSpacing);

            uint256 numTicks = 0;
            for (int256 word = wordPosLower; word <= wordPosUpper; word++) {
                uint256 bitmap = IStateViewTicks(stateView).getTickBitmap(poolId, int16(word));
                if (bitmap == 0) continue;
                for (uint256 bit; bit < 256; bit++) {
                    if (bitmap & (1 << bit) > 0) numTicks++;
                }
            }

            ticks = new Tick[](numTicks);
            uint256 idx = 0;
            for (int256 word = wordPosLower; word <= wordPosUpper; word++) {
                uint256 bitmap = IStateViewTicks(stateView).getTickBitmap(poolId, int16(word));
                if (bitmap == 0) continue;
                for (uint256 bit; bit < 256; bit++) {
                    if (bitmap & (1 << bit) == 0) continue;
                    int24 tick = int24(int256((word << 8) + int256(bit))) * tickSpacing;
                    (ticks[idx].liquidityGross, ticks[idx].liquidityNet) =
                        IStateViewTicks(stateView).getTickLiquidity(poolId, tick);
                    ticks[idx].index = tick;
                    idx++;
                }
            }
        }

        function _getTickSpacing(address positionManager, bytes25 poolIdPrefix)
            internal
            view
            returns (int24 tickSpacing)
        {
            (, , , tickSpacing,) = IPositionManagerPoolKeys(positionManager).poolKeys(poolIdPrefix);
        }

        function _getWordPositions(int24 tickSpacing) internal pure returns (int16 wordPosLower, int16 wordPosUpper) {
            int24 compressed = _compress(MIN_TICK, tickSpacing);
            wordPosLower = int16(compressed >> 8);
            compressed = _compress(MAX_TICK, tickSpacing);
            wordPosUpper = int16(compressed >> 8);
        }

        function _compress(int24 tick, int24 tickSpacing) internal pure returns (int24 compressed) {
            compressed = tick / tickSpacing;
            if (tick < 0 && tick % tickSpacing != 0) compressed--;
        }
    }
}
