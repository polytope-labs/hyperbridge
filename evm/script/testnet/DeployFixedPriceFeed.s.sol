// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";

import {BaseScript} from "../BaseScript.sol";
import {FixedPriceAggregator} from "./FixedPriceAggregator.sol";

/// @notice Deploys fixed-price native/USD and USDC/USD feeds for the testnet SimplexPaymaster
///         and writes them to NATIVE_ORACLE and USDC_ORACLE.
contract DeployScript is BaseScript {
    uint8 internal constant FEED_DECIMALS = 8;
    int256 internal constant USDC_USD = 1e8;

    function deploy() internal override {
        FixedPriceAggregator nativeFeed = new FixedPriceAggregator{salt: salt}(nativeUsd(), FEED_DECIMALS);
        FixedPriceAggregator usdcFeed = new FixedPriceAggregator{salt: salt}(USDC_USD, FEED_DECIMALS);

        vm.stopBroadcast();

        console.log("Native/USD feed deployed at:", address(nativeFeed));
        console.log("USDC/USD feed deployed at:", address(usdcFeed));

        config.set("NATIVE_ORACLE", address(nativeFeed));
        config.set("USDC_ORACLE", address(usdcFeed));
    }

    function nativeUsd() internal view returns (int256) {
        if (block.chainid == 97) return 600e8; // BNB
        if (block.chainid == 80002) return 0.25e8; // POL
        revert("no fixed native price for this chain");
    }
}
