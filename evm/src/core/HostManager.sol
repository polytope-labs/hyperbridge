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
import {PostRequest, GetRequest, GetResponse} from "@hyperbridge/core/libraries/Message.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {IncomingPostRequest, IApp} from "@hyperbridge/core/interfaces/IApp.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";

import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

import {HostParams, IHostManager, WithdrawParams} from "./EvmHost.sol";

/// Host manager params
struct HostManagerParams {
    /// The only relayer whose deliveries `onAccept` accepts, and the only account that may call
    /// `init`. Never zero. Rotated by a `SetAdmin` request from Hyperbridge.
    address admin;
    /// Local ismp host. Zero until `init` binds it.
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
        SetHostParam,
        SetAdmin
    }

    HostManagerParams private _params;

    // @dev Action is unauthorized
    error UnauthorizedAction();

    // @dev The message was delivered by a relayer other than the admin
    error UnauthorizedRelayer();

    // @dev The host is already bound
    error AlreadyInitialized();

    // @dev The admin may not be zero. It is the only account able to deliver governance here, so
    // a zero admin would leave no way to reach this contract again, rotation included.
    error InvalidAdmin();

    /**
     * @dev Emitted when a `SetAdmin` request replaces the admin
     * @param previous The admin before this change
     * @param current The admin from now on
     */
    event AdminUpdated(address previous, address current);

    // @dev restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (msg.sender != caller) revert UnauthorizedAction();
        _;
    }

    constructor(HostManagerParams memory managerParams) {
        if (managerParams.admin == address(0)) revert InvalidAdmin();
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

    /**
     * @notice The only relayer whose deliveries `onAccept` accepts: the admin
     */
    function relayer() external view returns (address) {
        return _params.admin;
    }

    /**
     * @notice Binds this contract to the ISMP host
     * @dev Exists to seal the cyclic dependency between this contract and the host when the host
     * is not known at construction; a manager constructed with its host set needs no `init`. Only
     * the admin may call it, and only once: the admin is the governance relayer key, and letting it
     * re-point the host later would let that key cut the host off from its own governance.
     * @param hostAddr The host this contract accepts `onAccept` calls from and acts upon
     */
    function init(address hostAddr) external restrict(_params.admin) {
        if (_params.host != address(0)) revert AlreadyInitialized();
        _params.host = hostAddr;
    }

    /**
     * @notice Applies a governance action from Hyperbridge to the host
     * @dev Only the host may call, only the admin may deliver, and only the Hyperbridge parachain
     * may send; any other relayer's delivery reverts and stays retryable by the admin. The first
     * byte of the body selects the action: `Withdraw` pays out fees, `SetHostParam` replaces the
     * host params, `SetAdmin` rotates the admin, refusing zero since it could never be rotated
     * back.
     * @param incoming The verified request and the relayer that submitted it
     */
    function onAccept(IncomingPostRequest calldata incoming) external override restrict(_params.host) {
        // Only the admin may deliver here.
        if (incoming.relayer != _params.admin) revert UnauthorizedRelayer();
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
        } else if (action == OnAcceptActions.SetAdmin) {
            // Rotates the governance relayer.
            address newAdmin = abi.decode(request.body[1:], (address));
            if (newAdmin == address(0)) revert InvalidAdmin();
            emit AdminUpdated({previous: _params.admin, current: newAdmin});
            _params.admin = newAdmin;
        }
    }
}
