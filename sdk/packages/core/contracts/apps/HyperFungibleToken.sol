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

/**
 * @title HyperFungibleToken
 * @notice An cross-chain fungible token implementation that can only be minted and burned by a designated gateway address.
 * @dev This abstract contract extends OpenZeppelin's ERC20 implementation and adds gateway-restricted minting and burning capabilities.
 * It is designed to be used in cross-chain bridge scenarios where a trusted gateway manages token supply.
 * Derived contracts must implement the gateway() function to return their specific gateway address.
 */
abstract contract HyperFungibleToken is ERC20 {
    /// @notice Custom error thrown when a non-gateway address attempts to mint or burn
    error OnlyGateway();

    /**
     * @notice Restricts function access to only the gateway address
     * @dev Reverts with OnlyGateway error if called by any address other than the gateway
     */
    modifier onlyGateway() {
        if (msg.sender != gateway()) revert OnlyGateway();
        _;
    }

    /**
     * @notice Initializes the token with a name and symbol
     * @param name The name of the token
     * @param symbol The symbol of the token
     */
    constructor(string memory name, string memory symbol) ERC20(name, symbol) {}

    /**
     * @notice Returns the gateway address
     * @dev Must be implemented by derived contracts to return their specific gateway address
     * The `gateway` should be an immutable contract that will never change.
     * @return The address of the gateway contract
     */
    function gateway() public view virtual returns (address);

    /**
     * @notice Mints tokens to the specified account
     * @dev Can only be called by the gateway address
     * @param to The address that will receive the minted tokens
     * @param amount The amount of tokens to mint
     */
    function mint(address to, uint256 amount) external onlyGateway {
        _mint(to, amount);
    }

    /**
     * @notice Burns tokens from the specified account
     * @dev Can only be called by the gateway address. The account must have sufficient balance.
     * @param from The address from which tokens will be burned
     * @param amount The amount of tokens to burn
     */
    function burn(address from, uint256 amount) external onlyGateway {
        _burn(from, amount);
    }
}
