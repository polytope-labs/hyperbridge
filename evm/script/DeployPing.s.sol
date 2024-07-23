// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";

import {PingModule} from "../examples/PingModule.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        string memory host = vm.envString("HOST");

        vm.startBroadcast(uint256(privateKey));
        PingModule ping = new PingModule{salt: salt}(admin);

        if (equal(host, "sepolia") || equal(host, "ethereum")) {
            ping.setIsmpHost(SEPOLIA_HOST);
        } else if (equal(host, "arbitrum-sepolia")) {
            ping.setIsmpHost(ARB_SEPOLIA_HOST);
        } else if (equal(host, "optimism-sepolia")) {
            ping.setIsmpHost(OP_SEPOLIA_HOST);
        } else if (equal(host, "base-sepolia")) {
            ping.setIsmpHost(BASE_SEPOLIA_HOST);
        } else if (equal(host, "bsc-testnet")) {
            ping.setIsmpHost(BSC_TESTNET_HOST);
        }
        vm.stopBroadcast();
    }
}
