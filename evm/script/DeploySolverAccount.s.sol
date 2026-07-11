// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";

import {BaseScript} from "./BaseScript.sol";
import {IntentGatewayV2} from "../src/apps/IntentGatewayV2.sol";
import {SolverAccount} from "../src/apps/intentsv2/SolverAccount.sol";

/// @notice Redeploys only the SolverAccount delegate against the existing
///         IntentGatewayV2. Used when the account's validation logic changes —
///         solver EOAs re-delegate to the new address via EIP-7702; the gateway
///         is untouched.
contract DeployScript is BaseScript {
    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // CREATE2 with the shared salt: the address depends only on
        // (bytecode, salt, gateway address), all identical across chains, so
        // the new SolverAccount lands at the same address everywhere.
        address intentGateway = config.get("INTENT_GATEWAY_V2").toAddress();
        SolverAccount solverAccount = new SolverAccount{salt: salt}(intentGateway);

        vm.stopBroadcast();

        console.log("SolverAccount deployed at:", address(solverAccount));

        config.set("SOLVER_ACCOUNT", address(solverAccount));
    }
}
