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

import {HandlerV1} from "./HandlerV1.sol";
import {IHandlerV2} from "@hyperbridge/core/interfaces/IHandlerV2.sol";
import {IConsensusV2} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {IntermediateState, StateMachineHeight, StateCommitment} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {IHandler} from "@hyperbridge/core/interfaces/IHandler.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";

/**
 * @title The ISMP Message Handler V2.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Extends HandlerV1 with batch call support and IConsensusV2 integration.
 * Relayers can bundle multiple handler operations into a single transaction via batchCall.
 * Also tracks which relayer submitted the consensus proof for each authority set epoch.
 */
contract HandlerV2 is HandlerV1, IHandlerV2 {
    // Maps authority set ID to the relayer that submitted the consensus proof for that epoch
    mapping(uint256 => address) private _relayers;

    // The current authority set epoch
    uint256 private _currentEpoch;

    error BatchCallFailed(uint256 index, bytes reason);
    event RelayerRegistered(uint256 indexed authoritySetId, address indexed relayer);
    event BatchExecuted(address indexed relayer, uint256 callCount);

    // The provided epoch is not exactly prevEpoch + 1
    error InvalidEpoch(uint256 expected, uint256 actual);

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IHandlerV2).interfaceId || super.supportsInterface(interfaceId);
    }

    /**
     * @dev Process a batch of encoded handler calls in a single transaction.
     * Uses delegatecall to self so msg.sender is preserved and storage writes
     * happen in this contract's context. Atomic, any failure reverts the entire batch.
     * @param calls - array of ABI-encoded handler function calls
     */
    function batchCall(bytes[] memory calls) external {
        uint256 len = calls.length;
        for (uint256 i = 0; i < len; ++i) {
            (bool success, bytes memory returnData) = address(this).delegatecall(calls[i]);
            if (!success) {
                revert BatchCallFailed(i, returnData);
            }
        }
        emit BatchExecuted(msg.sender, len);
    }

    /**
     * @dev Handle incoming consensus messages using IConsensusV2.
     * Verifies the proof, stores the new consensus state and intermediate states,
     * and records the relayer for the new authority set epoch if one occurred.
     * @param host - `IsmpHost`
     * @param proof - consensus proof, ABI-encoded as (bytes proofId, bytes consensusProof)
     */
    function handleConsensus(IHost host, bytes calldata proof) external override(HandlerV1, IHandler) notFrozen(host) {
        uint256 delay = block.timestamp - host.consensusUpdateTime();
        if (delay >= host.unStakingPeriod()) revert ConsensusClientExpired();

        (bytes memory proofId, bytes memory consensusProof) = abi.decode(proof, (bytes, bytes));

        (bytes memory verifiedState, IntermediateState[] memory intermediates, uint256 newEpoch) =
            IConsensusV2(host.consensusClient()).verify(proofId, consensusProof);
        host.storeConsensusState(verifiedState);

        uint256 intermediatesLen = intermediates.length;
        for (uint256 i = 0; i < intermediatesLen; i++) {
            IntermediateState memory intermediate = intermediates[i];
            uint256 latestHeight = host.latestStateMachineHeight(intermediate.stateMachineId);
            if (latestHeight != 0 && intermediate.height > latestHeight) {
                StateMachineHeight memory stateMachineHeight =
                    StateMachineHeight({stateMachineId: intermediate.stateMachineId, height: intermediate.height});
                host.storeStateMachineCommitment(stateMachineHeight, intermediate.commitment);
            }
        }

        if (newEpoch != 0) {
            uint256 expected = _currentEpoch + 1;
            if (newEpoch != expected) revert InvalidEpoch(expected, newEpoch);
            _currentEpoch = newEpoch;
            _relayers[newEpoch] = msg.sender;
            emit RelayerRegistered(newEpoch, msg.sender);
        }
    }

    /**
     * @dev Returns the relayer address for a given authority set ID.
     * @param authoritySetId - the authority set / epoch ID
     * @return the relayer address, or address(0) if not set
     */
    function relayerOf(uint256 authoritySetId) external view returns (address) {
        return _relayers[authoritySetId];
    }

    /**
     * @dev Returns the current authority set epoch.
     */
    function currentEpoch() external view returns (uint256) {
        return _currentEpoch;
    }
}
