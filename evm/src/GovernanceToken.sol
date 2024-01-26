// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {AccessControl} from "openzeppelin/access/AccessControl.sol";
import {ERC20} from "openzeppelin/token/ERC20/extensions/ERC20Permit.sol";

contract GovernanceToken is AccessControl, ERC20 {
    // roles
    bytes32 constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 constant BURNER_ROLE = keccak256("BURNER_ROLE");

    constructor(string memory _name, string memory _symbol, address _defaultOwner) ERC20(_name, _symbol) {
        _grantRole(DEFAULT_ADMIN_ROLE, _defaultOwner);
    }

    function setDefaultAdminRole(address account, bool grant) external onlyRole(DEFAULT_ADMIN_ROLE) {
        grant ? _grantRole(DEFAULT_ADMIN_ROLE, account) : _revokeRole(DEFAULT_ADMIN_ROLE, account);
    }

    function setMinterRole(address account, bool grant) external onlyRole(DEFAULT_ADMIN_ROLE) {
        grant ? _grantRole(MINTER_ROLE, account) : _revokeRole(MINTER_ROLE, account);
    }

    function setBurnerRole(address account, bool grant) external onlyRole(DEFAULT_ADMIN_ROLE) {
        grant ? _grantRole(BURNER_ROLE, account) : _revokeRole(BURNER_ROLE, account);
    }

    function mint(address account, uint256 amount) external onlyRole(MINTER_ROLE) returns (bool) {
        _mint(account, amount);
        return true;
    }

    function burn(address account, uint256 amount) external onlyRole(BURNER_ROLE) returns (bool) {
        _burn(account, amount);
        return true;
    }
}
