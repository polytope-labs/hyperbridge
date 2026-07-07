// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../src/utils/SimplexPaymaster.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        address nativeOracleAddr = config.get("NATIVE_ORACLE").toAddress();
        uint256 markupBps = vm.envOr("MARKUP_BPS", uint256(200)); // default 2%
        address treasury = vm.envOr("TREASURY", admin); // default to owner

        SimplexPaymaster paymaster = new SimplexPaymaster{salt: salt}(
            AggregatorV3Interface(nativeOracleAddr),
            markupBps,
            treasury,
            admin
        );

        console.log("SimplexPaymaster deployed at:", address(paymaster));
        console.log("  nativeOracle:", nativeOracleAddr);
        console.log("  markupBps:", markupBps);
        console.log("  treasury:", treasury);
        console.log("  owner:", admin);

        address usdcToken = config.get("USDC_TOKEN").toAddress();
        address usdcOracle = config.get("USDC_ORACLE").toAddress();
        paymaster.registerToken(usdcToken, AggregatorV3Interface(usdcOracle));
        console.log("  Registered USDC:", usdcToken, "oracle:", usdcOracle);

        if (config.exists("USDT_TOKEN") && config.exists("USDT_ORACLE")) {
            address usdtToken = config.get("USDT_TOKEN").toAddress();
            address usdtOracle = config.get("USDT_ORACLE").toAddress();
            paymaster.registerToken(usdtToken, AggregatorV3Interface(usdtOracle));
            console.log("  Registered USDT:", usdtToken, "oracle:", usdtOracle);
        }

        config.set("SIMPLEX_PAYMASTER", address(paymaster));

        console.log("");
        console.log("=== IMPORTANT: Post-deployment steps ===");
        console.log("1. Fund the EntryPoint deposit for the paymaster:");
        console.log("   cast send <ENTRY_POINT> \"depositTo(address)\" ", address(paymaster), " --value 0.01ether");
    }
}
