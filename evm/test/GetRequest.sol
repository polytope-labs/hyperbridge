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

import {BaseTest} from "./BaseTest.sol";
import {GetResponseMessage, GetTimeoutMessage, GetRequest, PostRequest, Message} from "@polytope-labs/ismp-solidity/Message.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";

contract GetRequestTest is BaseTest {
    using Message for PostRequest;
    using Message for GetRequest;

    function GetResponseNoChallengeNoTimeout(
        bytes memory consensusProof,
        GetRequest memory request,
        GetResponseMessage memory message
    ) public {
        feeToken.mint(address(testModule), 32 * host.perByteFee(StateMachine.evm(97)));
        bytes32 commitment = testModule.dispatch(request);
        assert(host.requestCommitments(commitment).sender == address(testModule));

        handler.handleConsensus(host, consensusProof);
        vm.warp(10);
        handler.handleGetResponses(host, message);

        assert(host.responseReceipts(commitment).relayer == address(this));
    }

    function GetTimeoutNoChallenge(
        bytes memory consensusProof,
        GetRequest memory request,
        GetTimeoutMessage memory message
    ) public {
        feeToken.mint(address(testModule), 32 * host.perByteFee(StateMachine.evm(97)));
        testModule.dispatch(request);
        request.timeoutTimestamp += uint64(block.timestamp);
        assert(host.requestCommitments(request.hash()).sender != address(0));

        handler.handleConsensus(host, consensusProof);
        vm.warp(1000);

        handler.handleGetRequestTimeouts(host, message);

        assert(host.requestCommitments(request.hash()).sender == address(0));
    }
}
