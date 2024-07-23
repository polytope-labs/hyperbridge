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

import {Bytes} from "@polytope-labs/solidity-merkle-trees/trie/Bytes.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout} from "@polytope-labs/ismp-solidity/Message.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";
import {BaseIsmpModule, IncomingPostRequest} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";

import {HostParams, IHostManager, WithdrawParams} from "../hosts/EvmHost.sol";
import {ICallDispatcher, CallDispatcherParams} from "../interfaces/ICallDispatcher.sol";

/// Host manager params
struct HostManagerParams {
    /// admin for setting the host address
    address admin;
    /// Local ismp host
    address host;
}

/**
 * @title The ISMP HostManager.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Allows cross-chain governance actions
 * for updating the ISMP Host parameters or withdrawing bridge revenue.
 */
contract HostManager is BaseIsmpModule {
    using Bytes for bytes;

    enum OnAcceptActions {
        Withdraw,
        SetHostParam
    }

    HostManagerParams private _params;

    modifier onlyIsmpHost() {
        require(msg.sender == _params.host, "HostManager: Unauthorized action");
        _;
    }

    modifier onlyAdmin() {
        require(msg.sender == _params.admin, "HostManager: Unauthorized action");
        _;
    }

    constructor(HostManagerParams memory managerParams) {
        _params = managerParams;
    }

    // Getter method for reading the host manager's params
    function params() public view returns (HostManagerParams memory) {
        return _params;
    }

    // This function can only be called once by the admin to set the IsmpHost.
    // This exists to seal the cyclic dependency between this contract & the ismp host.
    function setIsmpHost(address host) public onlyAdmin {
        _params.host = host;
        _params.admin = address(0);
    }

    function onAccept(IncomingPostRequest calldata incoming) external override onlyIsmpHost {
        PostRequest calldata request = incoming.request;
        // Only the Hyperbridge parachain can send requests to this module.
        require(request.source.equals(IIsmpHost(_params.host).hyperbridge()), "Unauthorized request");

        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.Withdraw) {
            // This is where governance & relayers can withdraw their revenue.
            WithdrawParams memory withdrawParams = abi.decode(request.body[1:], (WithdrawParams));
            IHostManager(_params.host).withdraw(withdrawParams);
        } else if (action == OnAcceptActions.SetHostParam) {
            HostParams memory hostParams = abi.decode(request.body[1:], (HostParams));
            IHostManager(_params.host).updateHostParams(hostParams);
        } else {
            revert("Unknown action");
        }
    }

    /*
     * @dev Do not send tokens to this contract, they cannot be recovered.
     */
    receive() external payable {}
}
