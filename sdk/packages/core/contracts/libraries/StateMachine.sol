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

import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";

/**
 * @title StateMachine
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Utility library for generating standardized state machine identifiers
 * @dev This library provides helper functions to create consistent identifiers for different
 * blockchain networks and state machines in the Hyperbridge protocol. Each blockchain type
 * has its own identifier format to ensure uniqueness across the ecosystem.
 * These identifiers are used throughout the protocol for routing messages between chains.
 */
library StateMachine {
    /// @notice The identifier for relay chains (Polkadot/Kusama relay chain)
    /// @dev Used to identify the main relay chain in the Polkadot ecosystem
    uint256 public constant RELAY_CHAIN = 0;

    /**
     * @notice Generate an identifier for a Polkadot parachain
     * @dev Creates a standardized identifier in the format "POLKADOT-{id}"
     * @param id The parachain ID on the Polkadot network
     * @return The formatted state machine identifier as bytes
     * @custom:example polkadot(1000) returns "POLKADOT-1000"
     */
    function polkadot(uint256 id) internal pure returns (bytes memory) {
        return bytes(string.concat("POLKADOT-", Strings.toString(id)));
    }

    /**
     * @notice Generate an identifier for a Kusama parachain
     * @dev Creates a standardized identifier in the format "KUSAMA-{id}"
     * @param id The parachain ID on the Kusama network
     * @return The formatted state machine identifier as bytes
     * @custom:example kusama(2000) returns "KUSAMA-2000"
     */
    function kusama(uint256 id) internal pure returns (bytes memory) {
        return bytes(string.concat("KUSAMA-", Strings.toString(id)));
    }

    /**
     * @notice Generate an identifier for an EVM-compatible chain
     * @dev Creates a standardized identifier in the format "EVM-{chainId}"
     * @param chainid The chain ID of the EVM network (e.g., 1 for Ethereum mainnet)
     * @return The formatted state machine identifier as bytes
     * @custom:example evm(1) returns "EVM-1" for Ethereum mainnet
     */
    function evm(uint256 chainid) internal pure returns (bytes memory) {
        return bytes(string.concat("EVM-", Strings.toString(chainid)));
    }

    /**
     * @notice Generate an identifier for a Substrate-based chain
     * @dev Creates a standardized identifier in the format "SUBSTRATE-{id}"
     * @param id The 4-byte identifier for the Substrate chain
     * @return The formatted state machine identifier as bytes
     * @custom:example substrate(0x12345678) returns "SUBSTRATE-0x12345678"
     */
    function substrate(bytes4 id) internal pure returns (bytes memory) {
        return bytes(string.concat("SUBSTRATE-", string(abi.encodePacked(id))));
    }
}
