// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {WrappedHyperFungibleToken} from "@hyperbridge/core/apps/WrappedHyperFungibleToken.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployWrappedHFT is BaseScript {
    function deploy() internal override {
        address underlying = vm.envAddress("UNDERLYING");
        bool isWeth = vm.envBool("IS_WETH");

        CallDispatcher dispatcher = new CallDispatcher{salt: salt}();
        WrappedHyperFungibleToken whft = new WrappedHyperFungibleToken{salt: salt}(admin);

        whft.configure(WrappedHyperFungibleToken.WrappedConfigOptions({
            host: HOST_ADDRESS,
            dispatcher: address(dispatcher),
            underlying: underlying,
            isWeth: isWeth
        }));

        vm.stopBroadcast();
        console.log("=== WrappedHFT Deployment ===");
        console.log("WrappedHyperFungibleToken:", address(whft));
        console.log("CallDispatcher:", address(dispatcher));
        console.log("Underlying:", underlying);
        console.log("IsWETH:", isWeth);
    }
}
