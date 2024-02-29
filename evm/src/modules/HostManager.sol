// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout} from "ismp/Message.sol";
import {StateMachine} from "ismp/StateMachine.sol";

import {HostParams, IHostManager, WithdrawParams} from "../hosts/EvmHost.sol";
import {BaseIsmpModule} from "ismp/IIsmpModule.sol";

/// Host manager params
struct HostManagerParams {
    /// admin for setting the host address
    address admin;
    /// Local ismp host
    address host;
    /// Hyperbridge state machine identifier
    bytes hyperbridge;
}

/// Manages the IsmpHost, allows cross-chain governance to configure params
/// and withdraw accrued revenue
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

    function onAccept(PostRequest calldata request) external override onlyIsmpHost {
        // Only Hyperbridge can send requests to this module.
        require(request.source.equals(_params.hyperbridge), "Unauthorized request");

        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.Withdraw) {
            // This is where governance & relayers can withdraw their revenue.
            WithdrawParams memory withdrawParams = abi.decode(request.body[1:], (WithdrawParams));
            IHostManager(_params.host).withdraw(withdrawParams);
        } else if (action == OnAcceptActions.SetHostParam) {
            HostParams memory hostParams = abi.decode(request.body[1:], (HostParams));
            IHostManager(_params.host).setHostParams(hostParams);
        } else {
            revert("Unknown action");
        }
    }
}
