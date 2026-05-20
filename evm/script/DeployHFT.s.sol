// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployHFT is BaseScript {
    function deploy() internal override {
        string memory name = vm.envString("HFT_NAME");
        string memory symbol = vm.envString("HFT_SYMBOL");

        CallDispatcher dispatcher = new CallDispatcher{salt: salt}();
        HyperFungibleToken hft = new HyperFungibleToken{salt: salt}(name, symbol, admin);

        hft.configure(HyperFungibleToken.ConfigOptions({
            host: HOST_ADDRESS,
            dispatcher: address(dispatcher)
        }));

        vm.stopBroadcast();
        console.log("=== HFT Deployment ===");
        console.log("HyperFungibleToken:", address(hft));
        console.log("CallDispatcher:", address(dispatcher));
    }
}
