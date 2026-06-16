// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

contract ConfigureHFTPeers is Script {
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
