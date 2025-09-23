// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {CrossChainInscription} from "../src/modules/Inscriptions.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        vm.startBroadcast(uint256(privateKey));
        CrossChainInscription module = new CrossChainInscription{salt: salt}(admin);
        module.setHost(HOST_ADDRESS);

        vm.stopBroadcast();
    }
}
