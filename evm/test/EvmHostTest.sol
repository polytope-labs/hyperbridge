// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {BaseTest} from "./BaseTest.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import "forge-std/Test.sol";
import "../src/hosts/EvmHost.sol";

contract EvmHostTest is BaseTest {
    using Bytes for bytes;

    // we should only be able to set consensus state multiple times on testnet
    function testSetConsensusState() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);

        // we can set consensus state
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"deadbeef");
        assert(host.consensusState().equals(hex"deadbeef"));

        // as many times as we want
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"beefdead");
        assert(host.consensusState().equals(hex"beefdead"));

        // reset it
        vm.prank(host.hostParams().admin);
        host.setConsensusState(new bytes(0));
        assert(host.consensusState().equals(new bytes(0)));

        // set chain Id to mainnet
        vm.chainId(host.chainId());
        // we can set consensus state
        vm.prank(host.hostParams().admin);
        host.setConsensusState(hex"beef");
        assert(host.consensusState().equals(hex"beef"));

        // but not anymore
        vm.startPrank(host.hostParams().admin);
        vm.expectRevert("Unauthorized action");
        host.setConsensusState(hex"feeb");
        assert(host.consensusState().equals(hex"beef"));
    }

    function testSetHostParamsAdmin() public {
        // set chain Id to testnet
        vm.chainId(host.chainId() + 5);
        assert(host.chainId() + 5 == block.chainid);

        // we can set host params
        vm.startPrank(host.hostParams().admin);
        host.setHostParamsAdmin(host.hostParams());
    }
}
