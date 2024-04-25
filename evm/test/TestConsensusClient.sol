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
pragma solidity 0.8.17;

import {IConsensusClient, IntermediateState} from "ismp/IConsensusClient.sol";

/// Test consensus client, performs no verification
contract TestConsensusClient is IConsensusClient {
    function verifyConsensus(bytes memory consensusState, bytes memory proof)
        external
        pure
        returns (bytes memory, IntermediateState memory)
    {
        IntermediateState memory intermediate = abi.decode(proof, (IntermediateState));

        return (consensusState, intermediate);
    }
}
