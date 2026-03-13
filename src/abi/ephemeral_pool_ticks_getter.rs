use alloy::sol;

sol! {

    interface IPool {
        function tickSpacing() external view returns (int24);

        function tickBitmap(int16 wordPos) external view returns (uint256);

        function ticks(int24 tick) external view returns (uint128 liquidityGross, int128 liquidityNet);
    }


    /// @notice A lens that fetches the `tickBitmap` for a Uniswap v3 pool without deployment
    /// @author Aperture Finance
    /// @dev The return data can be accessed externally by `eth_call` without a `to` address or internally by catching the
    /// revert data, and decoded by `abi.decode(data, (Slot[]))`
    #[sol(rpc, bytecode="60808060405260208161051a803803809161001a8285610492565b83398101031261048d57516001600160a01b0381169081900361048d576040516334324e9f60e21b8152602081600481855afa9081156102cf57600091610451575b5060020b6000811561043d575080620d89e7190560020b60081d60010b9080620d89e80560020b60081d60010b90600093835b8381131561038957506100a1856104d8565b946100af6040519687610492565b8086526100be601f19916104d8565b0160005b8181106103355750506000935b838113156101525785604051809163dff5870960e01b82526024820160206004840152815180915260206044840192019060005b818110610111575050500390fd5b8251805160020b85526020818101516001600160801b031681870152604091820151600f0b9186019190915286955060609094019390920191600101610103565b60405163299ce14b60e11b8152600182900b6004820152602081602481865afa9081156102cf57600091610304575b5080156102fa576000905b61010082106101a55750506101a0906104b5565b6100cf565b90956001871b8216156102f1578260081b87810190600089831291129080158216911516176102db57859060020b028060020b9081036102db576101e9828a6104ef565b51526101f581896104ef565b515160020b6040519063f30dba9360e01b82526004820152604081602481885afa9081156102cf578991600091829161026c575b5091839161026393604061024d85602061024560019a886104ef565b5101956104ef565b510190600f0b9052848060801b031690526104c9565b965b019061018c565b919250506040813d82116102c7575b8161028860409383610492565b810103126102c3578051906001600160801b03821682036102bf57602001519182600f0b83036102bc575089916001610229565b80fd5b8280fd5b5080fd5b3d915061027b565b6040513d6000823e3d90fd5b634e487b7160e01b600052601160045260246000fd5b95600190610265565b506101a0906104b5565b906020823d821161032d575b8161031d60209383610492565b810103126102bc57505138610181565b3d9150610310565b6040516060810191906001600160401b0383118184101761037357602092604052600081526000838201526000604082015282828a010152016100c2565b634e487b7160e01b600052604160045260246000fd5b60405163299ce14b60e11b8152600182900b6004820152602081602481865afa9081156102cf5760009161040c575b5080156104025760005b61010081106103db5750506103d6906104b5565b61008f565b6001811b82166103ee575b6001016103c2565b966103fa6001916104c9565b9790506103e6565b506103d6906104b5565b906020823d8211610435575b8161042560209383610492565b810103126102bc575051386103b8565b3d9150610418565b634e487b7160e01b81526012600452602490fd5b6020813d602011610485575b8161046a60209383610492565b810103126102c35751908160020b82036102bc57503861005c565b3d915061045d565b600080fd5b601f909101601f19168101906001600160401b0382119082101761037357604052565b6001600160ff1b0381146102db5760010190565b60001981146102db5760010190565b6001600160401b0381116103735760051b60200190565b80518210156105035760209160051b010190565b634e487b7160e01b600052603260045260246000fdfe")]
    #[derive(Debug)]
    contract EphemeralPoolTicksGetter {
        int24 internal constant MIN_TICK = -887272;
        int24 internal constant MAX_TICK = -MIN_TICK;

        #[derive(PartialEq, Eq, serde::Serialize)]
        struct Tick {
            int24 index;
            uint128 liquidityGross;
            int128 liquidityNet;
        }

        #[derive(PartialEq, Eq)]
        error Ticks(Tick[] ticks);


        constructor(address pool) payable {
            Tick[] memory ticks = getAllTicks(IPool(pool));
            // bytes memory returnData = abi.encode(ticks);
            revert Ticks(ticks);
            // assembly ("memory-safe") {
            //     revert(add(returnData, 0x20), mload(returnData))
            // }
        }

        function getAllTicks(IPool pool) public view returns (Tick[] memory ticks) {
            int24 tickSpacing = pool.tickSpacing();
            int256 minWord = int16((MIN_TICK / tickSpacing) >> 8);
            int256 maxWord = int16((MAX_TICK / tickSpacing) >> 8);

            uint256 numTicks = 0;
            for (int256 word = minWord; word <= maxWord; word++) {
                uint256 bitmap = pool.tickBitmap(int16(word));
                if (bitmap == 0) continue;
                for (uint256 bit; bit < 256; bit++) {
                    if (bitmap & (1 << bit) > 0) numTicks++;
                }
            }

            ticks = new Tick[](numTicks);
            uint256 idx = 0;
            for (int256 word = minWord; word <= maxWord; word++) {
                uint256 bitmap = pool.tickBitmap(int16(word));
                if (bitmap == 0) continue;
                for (uint256 bit; bit < 256; bit++) {
                    if (bitmap & (1 << bit) == 0) continue;
                    ticks[idx].index = int24((word << 8) + int256(bit)) * tickSpacing;
                    (ticks[idx].liquidityGross, ticks[idx].liquidityNet) = pool.ticks(ticks[idx].index);
                    idx++;
                }
            }
        }
    }

}

// #[test]
// fn error() {
//     assert_error_signature::<Ticks>("Ticks(Tick[] ticks)");

//     let call_data = hex!(
//         "0xdff5870900000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001"
//     );

//     assert_eq!(
//         Ticks::abi_decode_raw(&call_data),
//         Ok(Ticks {
//             ticks: vec![Tick {
//                 index: 1.try_into().expect("Failed to convert 1 to int24"),
//                 liquidityGross: 1,
//                 liquidityNet: 1,
//             }],
//         })
//     );
// }

// fn assert_call_signature<T: SolCall>(expected: &str) {
//     assert_eq!(T::SIGNATURE, expected);
//     assert_eq!(T::SELECTOR, keccak256(expected)[..4]);
// }

// fn assert_error_signature<T: SolError>(expected: &str) {
//     assert_eq!(T::SIGNATURE, expected);
//     assert_eq!(T::SELECTOR, keccak256(expected)[..4]);
// }
