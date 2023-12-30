// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";

import {IIsmpModule} from "ismp/IIsmpModule.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout} from "ismp/IIsmp.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {HostParams, IHostManager, WithdrawParams} from "../EvmHost.sol";

struct HostManagerParams {
    address admin;
    address host;
    uint256 paraId;
}

contract HostManager is IIsmpModule {
    using Bytes for bytes;

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

    function onAccept(PostRequest memory request) external onlyIsmpHost {
        // Only Hyperbridge can send requests to this module.
        require(request.source.equals(StateMachine.polkadot(_params.paraId)), "Unauthorized request");

        // TODO: we should decode the payload based on the first byte in the request.body
        if (false) {
            // This is where relayers can withdraw their fees.
            WithdrawParams memory params = abi.decode(request.body, (WithdrawParams));
            IHostManager(_params.host).withdraw(params);
        } else {
            HostParams memory params = abi.decode(request.body, (HostParams));
            IHostManager(_params.host).setHostParams(params);
        }
    }

    function onPostResponse(PostResponse memory response) external pure {
        revert("Module doesn't emit requests");
    }

    function onGetResponse(GetResponse memory response) external pure {
        revert("Module doesn't emit requests");
    }

    function onPostTimeout(PostRequest memory request) external pure {
        revert("Module doesn't emit requests");
    }

    function onGetTimeout(GetRequest memory request) external pure {
        revert("Module doesn't emit requests");
    }
}
