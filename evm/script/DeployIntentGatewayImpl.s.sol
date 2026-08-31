// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2} from "../src/apps/IntentGatewayV2.sol";
import {BaseScript} from "./BaseScript.sol";

/// @notice Deploys a new IntentGatewayV2 implementation only. The live ERC-1967 proxy
/// keeps its deterministic CREATE2 address; Hyperbridge governance points it at this
/// implementation via an UpgradeContract request (intents-coprocessor pallet's
/// upgrade_gateway extrinsic).
contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // The admin must match the owner the proxy was originally deployed with —
        // `_owner` is an immutable read from the implementation.
        IntentGatewayV2 implementation = new IntentGatewayV2{salt: salt}(admin);

        vm.stopBroadcast();

        console.log("IntentGatewayV2 implementation deployed at:", address(implementation));

        config.set("INTENT_GATEWAY_V2_IMPL", address(implementation));
    }
}
