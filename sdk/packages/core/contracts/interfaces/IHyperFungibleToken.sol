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

/**
 * @title IHyperFungibleToken
 * @notice Interface for cross-chain fungible tokens that are their own bridge application.
 * Shared by both HyperFungibleToken (burn/mint) and WrappedHyperFungibleToken (lock/unlock).
 */
interface IHyperFungibleToken {
    struct SendParams {
        bytes dest;
        bytes to;
        uint256 amount;
        uint64 timeout;
        uint256 relayerFee;
        bytes data;
    }

    struct ConfigOptions {
        address host;
        address dispatcher;
    }

    /// @notice Sends tokens cross-chain
    function send(SendParams calldata params) external payable;

    /// @notice Configures the host and dispatcher addresses (host is set-once)
    function configure(ConfigOptions calldata options) external;

    /// @notice Registers a supported chain and its peer module ID
    function addChain(bytes calldata chainId, bytes calldata moduleId) external;

    /// @notice Removes a chain from the supported set
    function removeChain(bytes calldata chainId) external;

    /// @notice Returns the peer module ID for a given chain
    function supportedChain(bytes calldata chainId) external view returns (bytes memory);

    /// @notice Returns the ISMP host address
    function host() external view returns (address);

    /// @notice Returns the CallDispatcher address
    function dispatcher() external view returns (address);

    /// @notice Pauses all cross-chain operations
    function pause() external;

    /// @notice Unpauses all cross-chain operations
    function unpause() external;
}

/**
 * @title IWrappedHyperFungibleToken
 * @notice Extended interface for wrapped cross-chain tokens that custody an underlying ERC20.
 */
interface IWrappedHyperFungibleToken is IHyperFungibleToken {
    struct WrappedConfigOptions {
        address host;
        address dispatcher;
        address underlying;
        bool isWeth;
    }

    /// @notice Configures the host, dispatcher, underlying token, and WETH flag (host is set-once)
    function configure(WrappedConfigOptions calldata options) external;

    /// @notice Returns the address of the underlying ERC20 token
    function underlying() external view returns (address);

    /// @notice Returns whether the underlying token is WETH
    function isWeth() external view returns (bool);
}
