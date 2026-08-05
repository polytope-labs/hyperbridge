// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {BridgeToken} from "../src/apps/BridgeToken.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployBridgeToken is BaseScript {
    function deploy() internal override {
        address dispatcher = config.get("CALL_DISPATCHER").toAddress();
        BridgeToken bridge = new BridgeToken{salt: salt}(admin);

        bridge.configure(HyperFungibleToken.ConfigOptions({host: HOST_ADDRESS, dispatcher: dispatcher}));

        vm.stopBroadcast();
        console.log("=== BridgeToken Deployment ===");
        console.log("BridgeToken:", address(bridge));
        console.log("CallDispatcher:", dispatcher);
        console.log("Nexus peer:", string(bridge.nexus()));
    }
}
