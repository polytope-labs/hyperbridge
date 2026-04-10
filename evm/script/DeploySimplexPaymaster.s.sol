// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../src/utils/SimplexPaymaster.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        // ── Read from TOML config ────────────────────────────────────
        address nativeOracleAddr = config.get("NATIVE_ORACLE").toAddress();
        uint256 markupBps = vm.envOr("MARKUP_BPS", uint256(200)); // default 2%
        address treasury = vm.envOr("TREASURY", admin); // default to owner

        // ── Deploy SimplexPaymaster ──────────────────────────────────
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

        // ── Register tokens from config ──────────────────────────────
        address usdcToken = config.get("USDC_TOKEN").toAddress();
        address usdcOracle = config.get("USDC_ORACLE").toAddress();
        address usdtToken = vm.envOr("USDT_TOKEN", address(0));
        address usdtOracle = vm.envOr("USDT_ORACLE", address(0));

        if (usdcToken != address(0) && usdcOracle != address(0)) {
            paymaster.registerToken(usdcToken, AggregatorV3Interface(usdcOracle));
            console.log("  Registered USDC:", usdcToken, "oracle:", usdcOracle);
        }

        if (usdtToken != address(0) && usdtOracle != address(0)) {
            paymaster.registerToken(usdtToken, AggregatorV3Interface(usdtOracle));
            console.log("  Registered USDT:", usdtToken, "oracle:", usdtOracle);
        }

        // ── Update config ────────────────────────────────────────────
        config.set("SIMPLEX_PAYMASTER", address(paymaster));

        console.log("");
        console.log("=== IMPORTANT: Post-deployment steps ===");
        console.log("1. Fund EntryPoint deposit for the paymaster:");
        console.log("   cast send <ENTRY_POINT> \"depositTo(address)\" ", address(paymaster), " --value 0.01ether");
    }
}
