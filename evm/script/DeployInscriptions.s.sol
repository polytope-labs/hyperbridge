// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {CrossChainInscription} from "../src/utils/Inscriptions.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        CrossChainInscription module = new CrossChainInscription{salt: salt}(admin);
        console.log("CrossChainInscription deployed at:", address(module));

        module.setHost(HOST_ADDRESS);
        console.log("Host configured");
    }
}
