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

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {AccessControlEnumerable} from "@openzeppelin/contracts/access/extensions/AccessControlEnumerable.sol";

/**
 * @title HyperFungibleTokenImpl
 * @notice Cross-chain fungible token with role-based access control
 * @dev This contract supports multiple minters and burners through role-based permissions.
 *      Useful for scenarios where both a TokenGateway and a TokenFaucet need minting capabilities.
 */
contract HyperFungibleTokenImpl is ERC20, AccessControlEnumerable {
    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    /// @notice Custom error thrown when a non-gateway address attempts to mint or burn
    error OnlyGateway();

    /**
     * @notice Initializes the token with a name, symbol, and admin
     * @param admin The address that will have DEFAULT_ADMIN_ROLE to grant/revoke roles
     * @param name The name of the token
     * @param symbol The symbol of the token
     */
    constructor(address admin, string memory name, string memory symbol) ERC20(name, symbol) {
        require(admin != address(0), "Admin cannot be zero address");
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
    }

    /**
     * @notice Returns the admin address (for compatibility with gateway() interface)
     * @dev Returns the address with DEFAULT_ADMIN_ROLE
     * @return The address of the admin
     */
    function gateway() public view returns (address) {
        // Return the first admin (there should typically be only one)
        // This is primarily for interface compatibility
        if (getRoleMemberCount(DEFAULT_ADMIN_ROLE) > 0) {
            return getRoleMember(DEFAULT_ADMIN_ROLE, 0);
        }
        return address(0);
    }

    /**
     * @notice Mints tokens to the specified account
     * @dev Can be called by any address with MINTER_ROLE
     * @param to The address that will receive the minted tokens
     * @param amount The amount of tokens to mint
     */
    function mint(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        _mint(to, amount);
    }

    /**
     * @notice Burns tokens from the specified account
     * @dev Can be called by any address with BURNER_ROLE
     * @param from The address from which tokens will be burned
     * @param amount The amount of tokens to burn
     */
    function burn(address from, uint256 amount) external onlyRole(BURNER_ROLE) {
        _burn(from, amount);
    }

    /**
     * @notice Grants minter role to an address
     * @param account The address to grant the minter role to
     */
    function grantMinterRole(address account) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _grantRole(MINTER_ROLE, account);
    }

    /**
     * @notice Grants burner role to an address
     * @param account The address to grant the burner role to
     */
    function grantBurnerRole(address account) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _grantRole(BURNER_ROLE, account);
    }

    /**
     * @notice Revokes minter role from an address
     * @param account The address to revoke the minter role from
     */
    function revokeMinterRole(address account) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _revokeRole(MINTER_ROLE, account);
    }

    /**
     * @notice Revokes burner role from an address
     * @param account The address to revoke the burner role from
     */
    function revokeBurnerRole(address account) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _revokeRole(BURNER_ROLE, account);
    }

    /**
     * @notice Helper function for tests - approves unlimited tokens
     * @param owner The owner of the tokens
     * @param spender The spender address
     */
    function superApprove(address owner, address spender) public {
        _approve(owner, spender, type(uint256).max);
    }
}
