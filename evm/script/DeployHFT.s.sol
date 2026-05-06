// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

/// @dev Testnet HFT with owner mint for bootstrapping
contract TestnetHFT is HyperFungibleToken {
    constructor(string memory name, string memory symbol, uint256 initialSupply)
        HyperFungibleToken(name, symbol)
    {
        _mint(msg.sender, initialSupply);
    }
}

contract DeployHFT is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address host = vm.envAddress("HOST");
        uint256 peerChainId = vm.envUint("PEER_CHAIN_ID");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy CallDispatcher
        CallDispatcher dispatcher = new CallDispatcher();

        // Deploy HFT with 1M initial supply
        TestnetHFT hft = new TestnetHFT("Test HFT", "tHFT", 1_000_000 ether);

        // Configure host and dispatcher
        hft.configure(HyperFungibleToken.ConfigOptions({
            host: host,
            dispatcher: address(dispatcher)
        }));

        vm.stopBroadcast();

        console.log("=== Deployment ===");
        console.log("HyperFungibleToken:", address(hft));
        console.log("CallDispatcher:", address(dispatcher));
        console.log("Host:", host);
        console.log("Chain ID:", block.chainid);
        console.log("Peer Chain ID:", peerChainId);
    }
}

contract ConfigurePeers is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address hftAddress = vm.envAddress("HFT_ADDRESS");
        address peerAddress = vm.envAddress("PEER_ADDRESS");
        uint256 peerChainId = vm.envUint("PEER_CHAIN_ID");

        vm.startBroadcast(deployerPrivateKey);

        HyperFungibleToken hft = HyperFungibleToken(hftAddress);
        hft.addChain(
            StateMachine.evm(peerChainId),
            abi.encodePacked(peerAddress)
        );

        vm.stopBroadcast();

        console.log("=== Peer Configured ===");
        console.log("HFT:", hftAddress);
        console.log("Peer:", peerAddress);
        console.log("Peer Chain:", peerChainId);
    }
}
