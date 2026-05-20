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

import {IConsensusV2, IntermediateState} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

contract TestConsensusClientV2 is IConsensusV2, ERC165 {
    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IConsensusV2).interfaceId
            || super.supportsInterface(interfaceId);
    }

    /**
     * @dev IConsensusV2 implementation — used by HandlerV2.handleConsensus.
     *
     * The new consensus state is `abi.encode(previousState, nextAuthoritySetId)` so it
     * always differs from `previousState`, mirroring a real client that advances state
     * on a valid proof. Returning `previousState` unchanged would short-circuit the
     * epoch-attribution path in HandlerV2.
     */
    function verify(bytes calldata previousState, bytes calldata proof)
        external
        pure
        returns (bytes memory, IntermediateState[] memory, uint256)
    {
        (IntermediateState memory intermediate, uint256 nextAuthoritySetId) =
            abi.decode(proof, (IntermediateState, uint256));

        IntermediateState[] memory intermediates = new IntermediateState[](1);
        intermediates[0] = intermediate;

        bytes memory newState = abi.encode(previousState, nextAuthoritySetId);
        return (newState, intermediates, nextAuthoritySetId);
    }
}
