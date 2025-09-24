// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";

import {PingModule} from "../examples/PingModule.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        address admin = vm.envAddress("ADMIN");
        address tokenFaucet = vm.envAddress("TOKEN_FAUCET");

        vm.startBroadcast(uint256(privateKey));
        PingModule ping = new PingModule{salt: salt}(admin);
        ping.setIsmpHost(HOST_ADDRESS, tokenFaucet);

        vm.stopBroadcast();
    }
}
