// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SolverAccount} from "../src/utils/SolverAccount.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        // Get IntentGatewayV2 address from config
        address intentGatewayV2 = config.get("INTENT_GATEWAY_V2").toAddress();

        // Deploy SolverAccount
        SolverAccount solverAccount = new SolverAccount{salt: salt}(intentGatewayV2);

        console.log("SolverAccount deployed at:", address(solverAccount));
        console.log("  IntentGatewayV2:", intentGatewayV2);

        // Update config
        config.set("SOLVER_ACCOUNT", address(solverAccount));
    }
}
