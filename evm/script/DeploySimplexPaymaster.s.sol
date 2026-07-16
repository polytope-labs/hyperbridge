// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../src/utils/SimplexPaymaster.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        address nativeOracleAddr = config.get("NATIVE_ORACLE").toAddress();
        uint256 markupBps = vm.envOr("MARKUP_BPS", uint256(200)); // default 2%
        address treasury = vm.envOr("TREASURY", admin); // default to deployer
        // Stablecoin feeds on Ethereum and Base run a 24h heartbeat; a buffer over
        // 24h avoids transient StaleOraclePrice reverts on late pushes.
        uint256 maxOracleAge = vm.envOr("MAX_ORACLE_AGE", uint256(90_000));
        uint256 swapSlippageBps = vm.envOr("SWAP_SLIPPAGE_BPS", uint256(200)); // default 2%

        bool hasUsdt = config.exists("USDT_TOKEN") && config.exists("USDT_ORACLE");
        uint256 tokenCount = hasUsdt ? 2 : 1;
        address[] memory tokens = new address[](tokenCount);
        AggregatorV3Interface[] memory oracles = new AggregatorV3Interface[](tokenCount);
        tokens[0] = config.get("USDC_TOKEN").toAddress();
        oracles[0] = AggregatorV3Interface(config.get("USDC_ORACLE").toAddress());
        if (hasUsdt) {
            tokens[1] = config.get("USDT_TOKEN").toAddress();
            oracles[1] = AggregatorV3Interface(config.get("USDT_ORACLE").toAddress());
        }

        SimplexPaymaster implementation = new SimplexPaymaster{salt: salt}();
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (
                HOST_ADDRESS,
                SimplexPaymaster.Params({
                    nativeOracle: AggregatorV3Interface(nativeOracleAddr),
                    markupBps: markupBps,
                    treasury: treasury,
                    maxOracleAge: maxOracleAge,
                    swapSlippageBps: swapSlippageBps
                }),
                tokens,
                oracles
            )
        );
        ERC1967Proxy proxy = new ERC1967Proxy{salt: salt}(address(implementation), initData);
        SimplexPaymaster paymaster = SimplexPaymaster(payable(address(proxy)));

        console.log("SimplexPaymaster implementation deployed at:", address(implementation));
        console.log("SimplexPaymaster proxy deployed at:", address(paymaster));
        console.log("  host:", HOST_ADDRESS);
        console.log("  nativeOracle:", nativeOracleAddr);
        console.log("  markupBps:", markupBps);
        console.log("  maxOracleAge:", maxOracleAge);
        console.log("  treasury:", treasury);
        console.log("  swapSlippageBps:", swapSlippageBps);
        console.log("  Registered USDC:", tokens[0], "oracle:", address(oracles[0]));
        if (hasUsdt) {
            console.log("  Registered USDT:", tokens[1], "oracle:", address(oracles[1]));
        }

        config.set("SIMPLEX_PAYMASTER", address(paymaster));

        console.log("");
        console.log("=== IMPORTANT: Post-deployment steps ===");
        console.log("1. Fund the EntryPoint deposit for the paymaster:");
        console.log("   cast send <ENTRY_POINT> \"depositTo(address)\" ", address(paymaster), " --value 0.01ether");
    }
}
