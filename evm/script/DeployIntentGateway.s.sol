// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGateway, Params} from "../src/modules/IntentGateway.sol";
import {BaseScript} from "./BaseScript.sol";
import {CallDispatcher} from "../src/modules/CallDispatcher.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        address admin = vm.envAddress("ADMIN");

        vm.startBroadcast(uint256(privateKey));
        CallDispatcher callDispatcher = new CallDispatcher{salt: salt}();

        IntentGateway gateway = new IntentGateway{salt: salt}(admin);
        gateway.setParams(Params({host: HOST_ADDRESS, dispatcher: address(callDispatcher)}));

        vm.stopBroadcast();
        console.log("IntentGateway deployed at:", address(gateway));
    }
}
