// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "solidity-merkle-trees/trie/Bytes.sol";

import "ismp/IIsmpModule.sol";
import "ismp/IIsmpHost.sol";
import "ismp/StateMachine.sol";

struct GovernorParams {
    address admin;
    address host;
    uint256 paraId;
}

contract CrossChainGovernor is IIsmpModule {
    using Bytes for bytes;

    GovernorParams private _params;

    modifier onlyIsmpHost() {
        require(msg.sender == _params.host, "CrossChainGovernor: Invalid caller");
        _;
    }

    modifier onlyAdmin() {
        require(msg.sender == _params.admin, "CrossChainGovernor: Invalid caller");
        _;
    }

    constructor(GovernorParams memory params) {
        _params = params;
    }

    // This function can only be called once by the admin to set the IsmpHost.
    // This exists to seal the cyclic dependency between this contract & the ismp host.
    function setIsmpHost(address host) public onlyAdmin {
        _params.host = host;
        _params.admin = address(0);
    }

    function onAccept(PostRequest memory request) external onlyIsmpHost {
        require(request.source.equals(StateMachine.polkadot(_params.paraId)), "Unauthorized request");
        (
            address admin,
            address consensus,
            address handler,
            uint256 challengePeriod,
            uint256 unstakingPeriod,
            uint256 defaultTimeout
        ) = abi.decode(request.body, (address, address, address, uint256, uint256, uint256));

        BridgeParams memory params =
            BridgeParams(admin, consensus, handler, challengePeriod, unstakingPeriod, defaultTimeout);

        IIsmpHost(_params.host).setBridgeParams(params);
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
