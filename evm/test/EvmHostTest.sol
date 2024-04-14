// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {BaseTest} from "./BaseTest.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import "forge-std/Test.sol";
import "../src/hosts/EvmHost.sol";
import {DispatchPost} from "ismp/IDispatcher.sol";
import {StateMachine} from "ismp/StateMachine.sol";

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

    function testFundRequest() public {
        vm.prank(tx.origin);
        bytes32 commitment = host.dispatch(
            DispatchPost({
                body: new bytes(0),
                payer: tx.origin,
                fee: 0,
                dest: StateMachine.arbitrum(),
                timeout: 0,
                to: new bytes(0)
            })
        );

        assert(host.requestCommitments(commitment).fee == 0);
        vm.prank(tx.origin);
        host.fundRequest(commitment, 10 * 1e18);
        assert(host.requestCommitments(commitment).fee == 10 * 1e18);

        vm.expectRevert("Unknown request");
        vm.prank(tx.origin);
        host.fundRequest(keccak256(hex"dead"), 10 * 1e18);

        vm.expectRevert("User can only fund own requests");
        host.fundRequest(commitment, 10 * 1e18);
    }
}
