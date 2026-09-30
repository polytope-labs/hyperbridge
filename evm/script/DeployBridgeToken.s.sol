// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {BridgeToken} from "../src/apps/BridgeToken.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployBridgeToken is BaseScript {
    function deploy() internal override {
        address dispatcher = config.get("CALL_DISPATCHER").toAddress();
        // The only account allowed to deliver messages to the token, on every chain. Read here rather
        // than in BaseScript so the other scripts do not require it.
        address relayer = vm.envAddress("GOVERNANCE_RELAYER");
        BridgeToken bridge = new BridgeToken{salt: salt}(admin);

        // Set before `configure`: until the host is set nothing can reach `onAccept`, so the token
        // is never live without its relayer.
        bridge.setRelayer(relayer);
        bridge.configure(HyperFungibleToken.ConfigOptions({host: HOST_ADDRESS, dispatcher: dispatcher}));

        vm.stopBroadcast();
        console.log("=== BridgeToken Deployment ===");
        console.log("BridgeToken:", address(bridge));
        console.log("CallDispatcher:", dispatcher);
        console.log("Relayer:", relayer);
        console.log("Nexus peer:", string(bridge.nexus()));
    }
}
