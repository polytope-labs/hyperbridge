// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2} from "../src/apps/IntentGatewayV2.sol";
import {IntentGatewayScript} from "./IntentGatewayScript.sol";

/// @notice Deploys the IntentGatewayV2 modules and a new implementation only. The live ERC-1967
/// proxy keeps its deterministic CREATE2 address; Hyperbridge governance points it at this
/// implementation through the intents-coprocessor pallet: `upgrade_gateway` for a proxy still
/// on the pre-`Execute` implementation, `execute_on_gateway` carrying `upgradeToAndCall`
/// calldata afterwards.
contract DeployScript is IntentGatewayScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        IntentGatewayV2 implementation = _deployImplementation();

        vm.stopBroadcast();

        _recordImplementation(implementation);
    }
}
