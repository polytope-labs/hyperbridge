// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";

import {PingModule} from "../examples/PingModule.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes("gargantua-v1000000"));

    address public SEPOLIA_HOST = 0xA4a35A7b9eB3C5196a991E74123463238e9a8a16;
    address public ARB_SEPOLIA_HOST = 0x49eF4e81209becb5D9C790F67fA6dbf2ca2A48c7;
    address public OP_SEPOLIA_HOST = 0x70659AAE08099361c97586E178be05DE19175C3F;
    address public BASE_SEPOLIA_HOST = 0x3f93CEc2136bC6b48557bC9E96D1eEda2a4E5f0B;
    address public BSC_SEPOLIA_HOST = 0x775De0A63ADc53BFfc9F239Ad542Bf6a5f94Eeaa;

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
            ping.setIsmpHost(BSC_SEPOLIA_HOST);
        }
        vm.stopBroadcast();
    }
}
