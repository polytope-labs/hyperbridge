// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGateway, Params} from "../src/apps/IntentGateway.sol";
import {BaseScript} from "./BaseScript.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        IntentGateway intentGateway = new IntentGateway{salt: salt}(admin);
        console.log("IntentGateway deployed at:", address(intentGateway));

        intentGateway.setParams(Params({host: HOST_ADDRESS, dispatcher: config.get("CALL_DISPATCHER").toAddress()}));
        console.log("IntentGateway configured");

        config.set("INTENT_GATEWAY", address(intentGateway));
    }
}
