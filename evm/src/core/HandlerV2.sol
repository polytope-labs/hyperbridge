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
    /**
     * @notice Maps an authority set ID (epoch) to the relayer that first submitted
     * the consensus proof for that epoch. Used to attribute and reward the relayer
     * responsible for submitting a new validator set transition.
     */
    mapping(uint256 => address) private _epochs;

    /**
     * @notice The most recent authority set ID for which a consensus proof has been submitted.
     * Monotonically increasing; updated in handleConsensus when a new epoch is observed.
     */
    uint256 private _currentEpoch;

    /**
     * @notice Reverted when a delegatecall in batchCall fails.
     * @param index The zero-based position of the failed call in the batch.
     * @param reason The raw revert data from the failed delegatecall.
     */
    error BatchCallFailed(uint256 index, bytes reason);

    /**
     * @notice Emitted when a consensus proof introduces a new authority set epoch.
     * @param authoritySetId The new authority set ID.
     * @param relayer The address of the relayer that submitted the proof.
     */
    event NewEpoch(uint256 indexed authoritySetId, address indexed relayer);

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
            if (!success) revert BatchCallFailed(i, returnData);
        }
    }

    /**
     * @dev Handle incoming consensus messages using IConsensusV2.
     * Verifies the proof, stores the new consensus state and intermediate states,
     * and records the relayer for the new authority set epoch if one occurred.
     * @param host - `IsmpHost`
     * @param proof - consensus proof
     */
    function handleConsensus(IHost host, bytes calldata proof) external override(HandlerV1, IHandler) notFrozen(host) {
        uint256 delay = block.timestamp - host.consensusUpdateTime();
        if (delay >= host.unStakingPeriod()) revert ConsensusClientExpired();

        (bytes memory verifiedState, IntermediateState[] memory intermediates, uint256 nextAuthoritySetId) =
            IConsensusV2(host.consensusClient()).verify(host.consensusState(), proof);
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

        if (nextAuthoritySetId > _currentEpoch) {
            _currentEpoch = nextAuthoritySetId;
            _epochs[nextAuthoritySetId] = msg.sender;
            emit NewEpoch(nextAuthoritySetId, msg.sender);
        }
    }

    /**
     * @dev Returns the relayer address for a given authority set ID.
     * @param authoritySetId - the authority set / epoch ID
     * @return the relayer address, or address(0) if not set
     */
    function relayerOf(uint256 authoritySetId) external view returns (address) {
        return _epochs[authoritySetId];
    }

    /**
     * @dev Returns the current authority set epoch.
     */
    function currentEpoch() external view returns (uint256) {
        return _currentEpoch;
    }
}
