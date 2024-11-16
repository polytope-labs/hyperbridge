// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";

import {PingModule} from "../examples/PingModule.sol";
import {BaseScript} from "./BaseScript.sol";

// Mostly for verifying MNTs not deploying
contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        address tokenGateway = vm.envAddress("TOKEN_GATEWAY");
        string memory name = vm.envString("TOKEN_NAME");
        string memory symbol = vm.envString("TOKEN_SYMBOL");
        vm.startBroadcast(uint256(privateKey));

        new ERC6160Ext20{salt: keccak256(bytes(symbol))}(tokenGateway, name, symbol);
        vm.stopBroadcast();
    }
}
