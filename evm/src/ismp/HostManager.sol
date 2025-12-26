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

import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse} from "@hyperbridge/core/libraries/Message.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {IncomingPostRequest, IApp} from "@hyperbridge/core/interfaces/IApp.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";

import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

import {HostParams, IHostManager, WithdrawParams} from "../hosts/EvmHost.sol";

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
contract HostManager is HyperApp, ERC165 {
    using Bytes for bytes;

    enum OnAcceptActions {
        Withdraw,
        SetHostParam
    }

    HostManagerParams private _params;

    // @dev Action is unauthorized
    error UnauthorizedAction();

    // @dev restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (msg.sender != caller) revert UnauthorizedAction();
        _;
    }

    constructor(HostManagerParams memory managerParams) {
        _params = managerParams;
    }

    /*
     * @dev fallback function for tests. Do not send any tokens directly to this contract.
     */
    receive() external payable {}

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IApp).interfaceId || super.supportsInterface(interfaceId);
    }

    // Getter method for reading the host manager's params
    function params() public view returns (HostManagerParams memory) {
        return _params;
    }

    // Implementation of HyperApp's required host() function
    function host() public view override returns (address) {
        return _params.host;
    }

    // This function can only be called once by the admin to set the IsmpHost.
    // This exists to seal the cyclic dependency between this contract & the ismp host.
    function setIsmpHost(address hostAddr) public restrict(_params.admin) {
        _params.host = hostAddr;
        _params.admin = address(0);
    }

    function onAccept(IncomingPostRequest calldata incoming) external override restrict(_params.host) {
        PostRequest calldata request = incoming.request;
        // Only the Hyperbridge parachain can send requests to this module.
        if (!request.source.equals(IHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.Withdraw) {
            // This is where governance & relayers can withdraw their revenue.
            WithdrawParams memory withdrawParams = abi.decode(request.body[1:], (WithdrawParams));
            IHostManager(_params.host).withdraw(withdrawParams);
        } else if (action == OnAcceptActions.SetHostParam) {
            HostParams memory hostParams = abi.decode(request.body[1:], (HostParams));
            IHostManager(_params.host).updateHostParams(hostParams);
        }
    }
}
