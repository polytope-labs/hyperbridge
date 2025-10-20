// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {Config} from "forge-std/Config.sol";

abstract contract BaseScript is Script, Config {
    // ============= Environment Variables =============
    bytes32 internal privateKey = vm.envBytes32("PRIVATE_KEY");
    address internal admin = vm.envAddress("ADMIN");
    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));
    bytes internal consensusState = vm.envBytes("CONSENSUS_STATE");

    // ============= Config Variables =============
    address internal HOST_ADDRESS;

    function setUp() public {
        // Load config
        _loadConfig("./config.toml", false);
        
        HOST_ADDRESS = config.get("HOST").toAddress();
    }
}
