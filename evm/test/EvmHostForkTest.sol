// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import "forge-std/console.sol";

import {MainnetForkBaseTest} from "./MainnetForkBaseTest.sol";
import {TeleportParams, Body, BODY_BYTES_SIZE} from "../src/modules/TokenGateway.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";
import "@polytope-labs/ismp-solidity/IDispatcher.sol";
import "../src/hosts/EvmHost.sol";
import "@polytope-labs/ismp-solidity/Message.sol";

contract EvmHostForkTest is MainnetForkBaseTest {
    using Message for PostResponse;
    using Message for PostRequest;
    using Message for GetRequest;

    // Maximum slippage of 0.5%
    uint256 maxSlippagePercentage = 5; // 0.5 * 100
    // mainnet address holding eth and dai
    address whaleAccount = address(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045);

    function testCanDispatchPostRequestWithNative() public {
        // per-byte fee
        uint256 messagingFee = 32 * host.perByteFee(StateMachine.evm(421614));

        // dispatch request
        bytes32 commitment = host.dispatch{value: quote(messagingFee)}(
            DispatchPost({
                body: abi.encodePacked(bytes32(0)),
                payer: whaleAccount,
                fee: 0,
                dest: StateMachine.evm(421614),
                timeout: 0,
                to: abi.encode(bytes32(0))
            })
        );
        assert(host.requestCommitments(commitment).sender == whaleAccount);
    }

    function testCanDispatchPostResponseWithNative() public {
        // per-byte fee
        uint256 messagingFee = 32 * host.perByteFee(host.host());

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: new bytes(0),
            to: abi.encodePacked(address(manager)),
            timeoutTimestamp: 0,
            body: bytes.concat(hex"01", abi.encode(host.hostParams()))
        });

        vm.prank(address(handler));
        host.dispatchIncoming(request, address(this));
        assert(host.requestReceipts(request.hash()) == address(this));

        // dispatch response
        uint256 cost = quote(messagingFee);
        bytes memory response = abi.encode(bytes32(0));

        vm.prank(whaleAccount); // send some eth to the manager
        (bool ok, ) = address(manager).call{value: cost}("");
        if (!ok) revert("Transfer failed");

        vm.prank(address(manager));
        bytes32 commitment = host.dispatch{value: cost}(
            DispatchPostResponse({request: request, response: response, fee: 0, timeout: 0, payer: address(manager)})
        );
        assert(host.responseCommitments(commitment).sender == address(manager));
    }

    function testCanDispatchGetRequestWithNative() public {
        // per-byte fee
        uint256 messagingFee = 32 * host.perByteFee(StateMachine.evm(97));

        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encode(whaleAccount);

        // dispatch request
        uint256 cost = quote(messagingFee);
        vm.prank(whaleAccount);
        bytes32 commitment = host.dispatch{value: cost}(
            DispatchGet({
                dest: StateMachine.evm(97),
                height: 100,
                keys: keys,
                timeout: 60 * 60,
                context: new bytes(0),
                fee: messagingFee
            })
        );
        assert(host.requestCommitments(commitment).sender == whaleAccount);
    }

    function testCanDispatchFundRequestWithNative() public {
        // per-byte fee
        uint256 messagingFee = 32 * host.perByteFee(StateMachine.evm(97));

        // dispatch request
        vm.prank(whaleAccount);
        bytes32 commitment = host.dispatch{value: quote(messagingFee)}(
            DispatchPost({
                body: abi.encode(bytes32(0)),
                payer: whaleAccount,
                fee: 0,
                dest: StateMachine.evm(421614),
                timeout: 0,
                to: abi.encode(bytes32(0))
            })
        );
        assert(host.requestCommitments(commitment).sender == whaleAccount);
        assert(host.requestCommitments(commitment).fee == 0);

        // fund request
        vm.prank(whaleAccount);
        uint256 newfee = 10 * 1e18;
        host.fundRequest{value: quote(newfee)}(commitment, newfee);
        assert(host.requestCommitments(commitment).fee == newfee);

        // can't fund unknown requests
        uint256 cost = quote(newfee);
        vm.expectRevert(EvmHost.UnknownRequest.selector);
        vm.prank(whaleAccount);
        host.fundRequest{value: cost}(keccak256(hex"dead"), 10 * 1e18);
    }

    function testCanDispatchFundResponseWithNative() public {
        // per-byte fee
        uint256 messagingFee = 32 * host.perByteFee(StateMachine.evm(97));

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: new bytes(0),
            to: abi.encodePacked(address(manager)),
            timeoutTimestamp: 0,
            body: bytes.concat(hex"01", abi.encode(host.hostParams()))
        });

        vm.prank(address(handler));
        host.dispatchIncoming(request, address(this));
        assert(host.requestReceipts(request.hash()) == address(this));

        // dispatch response
        uint256 cost = quote(messagingFee);
        bytes memory response = abi.encode(bytes32(0));

        vm.prank(whaleAccount); // send some eth to the manager
        (bool ok, ) = address(manager).call{value: cost}("");
        if (!ok) revert("Transfer failed");

        vm.prank(address(manager));
        bytes32 commitment = host.dispatch{value: cost}(
            DispatchPostResponse({request: request, response: response, fee: 0, timeout: 0, payer: address(manager)})
        );
        assert(host.responseCommitments(commitment).sender == address(manager));
        assert(host.responseCommitments(commitment).fee == 0);

        // fund request
        vm.prank(whaleAccount);
        uint256 newfee = 10 * 1e18;
        host.fundResponse{value: quote(newfee)}(commitment, newfee);
        assert(host.responseCommitments(commitment).fee == newfee);

        // can't fund unknown requests
        uint256 newCost = quote(newfee);
        vm.expectRevert(EvmHost.UnknownResponse.selector);
        vm.prank(whaleAccount);
        host.fundResponse{value: newCost}(keccak256(hex"dead"), 10 * 1e18);
    }

    /*function testCanWithdrawNativeToken() public {
        // per-byte fee
        uint256 amount = 1 * 1e18;

        PostRequest memory request = PostRequest({
            source: host.hyperbridge(),
            dest: host.host(),
            nonce: 0,
            from: new bytes(0),
            to: abi.encodePacked(address(manager)),
            timeoutTimestamp: 0,
            body: bytes.concat(
                hex"00",
                abi.encode(WithdrawParams({beneficiary: address(manager), amount: amount, native: true}))
            )
        });

        assert(address(manager).balance == 0);

        vm.prank(whaleAccount); // send some eth to the manager
        (bool ok, ) = address(host).call{value: amount}("");
        if (!ok) revert("Transfer failed");

        vm.prank(address(handler));
        host.dispatchIncoming(request, address(this));
        assert(host.requestReceipts(request.hash()) == address(this));
        assert(address(manager).balance == amount);
    }*/

    /*function testCanPayForStateCommitment() public {
        HostParams memory params = host.hostParams();

        // create a state commitment
        StateMachineHeight memory height = StateMachineHeight({height: 100, stateMachineId: 2000});
        StateCommitment memory commitment = StateCommitment({
            timestamp: 200,
            overlayRoot: bytes32(0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff),
            stateRoot: bytes32(0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff)
        });
        vm.prank(params.handler);
        host.storeStateMachineCommitment(height, commitment);

        uint256 cost = quote(params.stateCommitmentFee);
        StateCommitment memory retrieved = host.stateMachineCommitment{value: cost}(height);
        assert(commitment.timestamp == retrieved.timestamp);
        assert(commitment.overlayRoot == retrieved.overlayRoot);
        assert(commitment.stateRoot == retrieved.stateRoot);

        vm.prank(whaleAccount);
        feeToken.approve(address(host), type(uint256).max);
        vm.prank(whaleAccount);
        StateCommitment memory withFeeToken = host.stateMachineCommitment(height);
        assert(commitment.timestamp == withFeeToken.timestamp);
        assert(commitment.overlayRoot == withFeeToken.overlayRoot);
        assert(commitment.stateRoot == withFeeToken.stateRoot);

        feeToken.approve(address(host), type(uint256).max);
        vm.expectRevert("Dai/insufficient-balance");
        host.stateMachineCommitment(height);
    }*/

    function quote(uint256 feeTokenCost) internal view returns (uint256) {
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(IIsmpHost(gateway.params().host).uniswapV2Router()).WETH();
        path[1] = address(feeToken);

        return _uniswapV2Router.getAmountsIn(feeTokenCost, path)[0];
    }

    function addressToBytes32(address _address) public pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }
}
