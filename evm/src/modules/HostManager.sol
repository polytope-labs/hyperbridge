// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout} from "ismp/IIsmp.sol";
import {StateMachine} from "ismp/StateMachine.sol";

import {HostParams, IHostManager, WithdrawParams} from "../hosts/EvmHost.sol";
import {BaseIsmpModule} from "./BaseIsmpModule.sol";

struct HostManagerParams {
    address admin;
    address host;
    uint256 paraId;
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
        require(msg.sender == _params.host, "CrossChainGovernor: Invalid caller");
        _;
    }

    modifier onlyAdmin() {
        require(msg.sender == _params.admin, "CrossChainGovernor: Invalid caller");
        _;
    }

    constructor(HostManagerParams memory params) {
        _params = params;
    }

    // This function can only be called once by the admin to set the IsmpHost.
    // This exists to seal the cyclic dependency between this contract & the ismp host.
    function setIsmpHost(address host) public onlyAdmin {
        _params.host = host;
        _params.admin = address(0);
    }

    function onAccept(PostRequest calldata request) external override onlyIsmpHost {
        // Only Hyperbridge can send requests to this module.
        require(request.source.equals(StateMachine.kusama(_params.paraId)), "Unauthorized request");

        // note, the below will revert with solidity error `Panic(0x21)`
        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.Withdraw) {
            // This is where relayers can withdraw their fees.
            WithdrawParams memory params = abi.decode(request.body[1:], (WithdrawParams));
            IHostManager(_params.host).withdraw(params);
        } else if (action == OnAcceptActions.SetHostParam) {
            HostParams memory params = abi.decode(request.body[1:], (HostParams));
            IHostManager(_params.host).setHostParams(params);
        } else {
            revert("Unknown action");
        }
    }
}
