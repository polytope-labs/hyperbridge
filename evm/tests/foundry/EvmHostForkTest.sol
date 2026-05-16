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
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import "@hyperbridge/core/interfaces/IDispatcher.sol";
import "../../src/core/EvmHost.sol";
import "@hyperbridge/core/libraries/Message.sol";

contract EvmHostForkTest is MainnetForkBaseTest {
    using Message for PostRequest;
    using Message for GetRequest;

    // Maximum slippage of 0.5%
    uint256 maxSlippagePercentage = 5; // 0.5 * 100
    // mainnet address holding eth and dai
    address whaleAccount = address(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045);

    function testCanDispatchPostRequestWithNative() public {
        uint256 relayerFee = 10 * 1e18;

        // dispatch request
        bytes32 commitment = host.dispatch{value: quote(relayerFee)}(
            DispatchPost({
                body: abi.encodePacked(bytes32(0)),
                payer: whaleAccount,
                fee: relayerFee,
                dest: StateMachine.evm(421614),
                timeout: 0,
                to: abi.encode(bytes32(0))
            })
        );
        assert(host.requestCommitments(commitment).sender == whaleAccount);
    }

    function testCanDispatchGetRequestWithNative() public {
        uint256 relayerFee = 10 * 1e18;

        bytes[] memory keys = new bytes[](1);
        keys[0] = abi.encode(whaleAccount);

        // dispatch request
        uint256 cost = quote(relayerFee);
        vm.prank(whaleAccount);
        bytes32 commitment = host.dispatch{value: cost}(
            DispatchGet({
                dest: StateMachine.evm(97),
                height: 100,
                keys: keys,
                timeout: 60 * 60,
                context: new bytes(0),
                fee: relayerFee
            })
        );
        assert(host.requestCommitments(commitment).sender == whaleAccount);
    }

    function testCanDispatchFundRequestWithNative() public {
        // dispatch request
        vm.prank(whaleAccount);
        bytes32 commitment = host.dispatch(
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

    function quote(uint256 feeTokenCost) internal view returns (uint256) {
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(IHost(address(host)).uniswapV2Router()).WETH();
        path[1] = address(feeToken);

        return _uniswapV2Router.getAmountsIn(feeTokenCost, path)[0];
    }

    function addressToBytes32(address _address) public pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }
}
