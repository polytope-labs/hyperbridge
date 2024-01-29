// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {AccessControl} from "openzeppelin/access/AccessControl.sol";
import {ERC20} from "openzeppelin/token/ERC20/extensions/ERC20Permit.sol";
import "ismp/IIsmpModule.sol";

contract GovernableToken is AccessControl, ERC20, IIsmpModule {
    // roles
    bytes32 constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 constant BURNER_ROLE = keccak256("BURNER_ROLE");
    bytes32 constant HOST_ROLE = keccak256("HOST_ROLE");

    constructor(string memory _name, string memory _symbol, address _defaultOwner) ERC20(_name, _symbol) {
        _grantRole(DEFAULT_ADMIN_ROLE, _defaultOwner);
    }

    function mint(address account, uint256 amount) external onlyRole(MINTER_ROLE) returns (bool) {
        _mint(account, amount);
        return true;
    }

    function burn(address account, uint256 amount) external onlyRole(BURNER_ROLE) returns (bool) {
        _burn(account, amount);
        return true;
    }

    function onAccept(PostRequest calldata request) external onlyRole(HOST_ROLE) {
        (address account, bytes32 role, bool grant) = abi.decode(request.body, (address, bytes32, bool));
        grant ? _grantRole(role, account) : _revokeRole(role, account);
    }

    function onPostResponse(PostResponse memory) external view onlyRole(HOST_ROLE) {
        revert("Token gateway doesn't emit Post responses");
    }

    function onGetResponse(GetResponse memory) external view onlyRole(HOST_ROLE) {
        revert("Token gateway doesn't emit Get responses");
    }

    function onPostRequestTimeout(PostRequest memory) external view onlyRole(HOST_ROLE) {
        revert("Token gateway doesn't emit Post responses");
    }

    function onPostResponseTimeout(PostResponse memory) external view onlyRole(HOST_ROLE) {
        revert("Token gateway doesn't emit Post responses");
    }

    function onGetTimeout(GetRequest memory) external view onlyRole(HOST_ROLE) {
        revert("Token gateway doesn't emit Get requests");
    }
}
