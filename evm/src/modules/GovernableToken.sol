// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {PostRequest} from "ismp/IIsmp.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";
import {BaseIsmpModule} from "./BaseIsmpModule.sol";

// This action is dispatched by hyperbridge's governance to modify the token's permitted minters & burners.
struct GovernableTokenAction {
    // account in question
    address account;
    // role in question
    bytes32 role;
    // if true will grant role, otherwise will revoke role
    bool grant;
}

contract GovernableToken is BaseIsmpModule, ERC6160Ext20 {
    // address of the local ismp host
    address private _host;

    constructor(address _defaultOwner, string memory _name, string memory _symbol)
        ERC6160Ext20(_defaultOwner, _name, _symbol)
    {}

    modifier onlyAdmin(bytes32 role) {
        require(_rolesAdmin[role][_msgSender()], "Unauthorized action");
        _;
    }

    modifier onlyIsmpHost() {
        require(_msgSender() == _host, "Unauthorized action");
        _;
    }

    // Will replace the previous admin, with the IsmpHost
    function setIsmpHost(address host) public onlyAdmin(MINTER_ROLE) {
        delete _rolesAdmin[MINTER_ROLE][_msgSender()];
        delete _rolesAdmin[BURNER_ROLE][_msgSender()];

        _rolesAdmin[MINTER_ROLE][host] = true;
        _rolesAdmin[BURNER_ROLE][host] = true;
        _host = host;
    }

    /// Receives governance actions to modify it's admins
    function onAccept(PostRequest calldata request) external override onlyIsmpHost {
        GovernableTokenAction memory body = abi.decode(request.body, (GovernableTokenAction));
        body.grant ? grantRole(body.role, body.account) : revokeRole(body.role, body.account);
    }
}
