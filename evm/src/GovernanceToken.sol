// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {ERC20} from "openzeppelin/token/ERC20/extensions/ERC20Permit.sol";
import "ismp/IIsmpModule.sol";
import "multi-chain-tokens/interfaces/IERC6160Ext20.sol";

contract GovernableToken is ERC20, IIsmpModule, IERC6160Ext20 {
    bytes32 constant DEFAULT_ADMIN_ROLE = 0x0;
    bytes32 constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 constant BURNER_ROLE = keccak256("BURNER_ROLE");
    bytes32 constant HOST_ROLE = keccak256("HOST_ROLE");

    mapping(bytes32 => mapping(address => bool)) public hasRole;

    constructor(string memory _name, string memory _symbol, address _defaultOwner) ERC20(_name, _symbol) {
        _grantRole(DEFAULT_ADMIN_ROLE, _defaultOwner);
    }

    modifier onlyRole(bytes32 role) {
        require(hasRole[role][msg.sender], "Does not have role"); 
        _;
    }

    function _grantRole(bytes32 role, address account) private onlyRole(DEFAULT_ADMIN_ROLE) {
        hasRole[role][account] = true;
    }

    function _revokeRole(bytes32 role, address account) private onlyRole(DEFAULT_ADMIN_ROLE) {
        hasRole[role][account] = false;
    }

    function mint(address account, uint256 amount, bytes calldata ) external onlyRole(MINTER_ROLE) {
        _mint(account, amount);
    }

    function burn(address account, uint256 amount, bytes calldata) external onlyRole(BURNER_ROLE)  {
        _burn(account, amount);
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



    function grantRole(bytes32, address) external pure {
        revert();

    }
    function revokeRole(bytes32, address) external pure{
        revert();
    }
}
