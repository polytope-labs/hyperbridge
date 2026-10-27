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

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {StateMachineHeight, StateCommitment} from "@hyperbridge/core/interfaces/IConsensusV2.sol";

import {HandlerV2} from "./HandlerV2.sol";

/**
 * @title The Ownable ISMP Message Handler.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice A HandlerV2 whose owner can set the host's consensus state without a consensus proof.
 * All message handling is inherited from HandlerV2 and stays open to any relayer.
 */
contract OwnableHandler is HandlerV2, Ownable {
    constructor(address admin) Ownable(admin) {}

    /**
     * @dev Sets the host's consensus state without a proof, and resets the latest height of
     * `height.stateMachineId` to `height.height` with `commitment` stored at that height.
     */
    function setConsensusState(
        IHost host,
        bytes memory state,
        StateMachineHeight memory height,
        StateCommitment memory commitment
    ) external onlyOwner {
        host.storeConsensusState(state);
        host.storeStateMachineCommitment(height, commitment);
    }
}
