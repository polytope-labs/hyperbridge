// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";

import {PingModule} from "../examples/PingModule.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes("gargantua-v1000000"));

    address public SEPOLIA_HOST = vm.envAddress("SEPOLIA_HOST");
    address public ARB_SEPOLIA_HOST = vm.envAddress("ARB_SEPOLIA_HOST");
    address public OP_SEPOLIA_HOST = vm.envAddress("OP_SEPOLIA_HOST");
    address public BASE_SEPOLIA_HOST = vm.envAddress("BASE_SEPOLIA_HOST");
    address public BSC_TESTNET_HOST = vm.envAddress("BSC_TESTNET_HOST");

    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        string memory host = vm.envString("HOST");

        vm.startBroadcast(uint256(privateKey));
        PingModule ping = new PingModule{salt: salt}(admin);

        if (Strings.equal(host, "sepolia") || Strings.equal(host, "ethereum")) {
            ping.setIsmpHost(SEPOLIA_HOST);
        } else if (Strings.equal(host, "arbitrum-sepolia")) {
            ping.setIsmpHost(ARB_SEPOLIA_HOST);
        } else if (Strings.equal(host, "optimism-sepolia")) {
            ping.setIsmpHost(OP_SEPOLIA_HOST);
        } else if (Strings.equal(host, "base-sepolia")) {
            ping.setIsmpHost(BASE_SEPOLIA_HOST);
        } else if (Strings.equal(host, "bsc-testnet")) {
            ping.setIsmpHost(BSC_TESTNET_HOST);
        }
        vm.stopBroadcast();
    }
}
