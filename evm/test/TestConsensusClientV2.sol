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

import {IConsensus, IntermediateState} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {IConsensusV2} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

contract TestConsensusClientV2 is IConsensus, IConsensusV2, ERC165 {
    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensus).interfaceId || interfaceId == type(IConsensusV2).interfaceId
            || super.supportsInterface(interfaceId);
    }

    /**
     * @dev IConsensus implementation — unused by HandlerV2.
     */
    function verifyConsensus(bytes memory consensusState, bytes memory proof)
        external
        pure
        returns (bytes memory, IntermediateState[] memory)
    {
        IntermediateState memory intermediate = abi.decode(proof, (IntermediateState));
        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;
        return (consensusState, intermediates);
    }

    /**
     * @dev IConsensusV2 implementation — used by HandlerV2.handleConsensus.
     */
    function verify(bytes memory, /* proofId */ bytes memory proof)
        external
        pure
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        (bytes memory newState, IntermediateState memory intermediate, uint256 newEpoch) =
            abi.decode(proof, (bytes, IntermediateState, uint256));

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        return (newState, intermediates, newEpoch);
    }
}
