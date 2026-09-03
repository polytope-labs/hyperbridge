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

import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

/**
 * @title BridgeToken
 * @author Polytope Labs (hello@polytope.technology)
 * @notice The EVM representation of BRIDGE, the native token of the nexus parachain.
 *
 * @dev BRIDGE is native to nexus, so the two ends run the escrow model: `pallet-hyper-fungible-token`
 * escrows the native balance on nexus and this contract mints the equivalent here, meaning the supply
 * of this token is always backed by the pallet's escrow account. Sending back burns here and releases
 * there.
 *
 * Metadata and the nexus peer are fixed in the bytecode rather than passed at deployment, so every
 * chain gets an identical token, and with CREATE2 an identical address for the same deployer and salt.
 *
 * `decimals()` is the inherited ERC20 default of 18 while BRIDGE is 12 decimals on nexus, so the
 * pallet scales by 10^6 in both directions. The chain config registered on nexus via `register_token`
 * must therefore declare 18 decimals for this contract.
 */
contract BridgeToken is HyperFungibleToken {
    /// @notice The parachain id of nexus
    uint256 public constant NEXUS_PARA_ID = 3367;

    /// @notice The module id of `pallet-hyper-fungible-token`, its 8 byte `pall_hft` PalletId
    bytes8 public constant NEXUS_MODULE_ID = bytes8("pall_hft");

    /**
     * @notice Deploys the token with nexus already registered as a peer
     * @param initialOwner The address that will own this contract. It can register further peers,
     * pause cross chain operations, and change the dispatcher, so it should be a multisig or a
     * governance controlled account.
     */
    constructor(address initialOwner) HyperFungibleToken("Hyperbridge", "BRIDGE", initialOwner) {
        _supportedChains[nexus()] = abi.encodePacked(NEXUS_MODULE_ID);
    }

    /**
     * @notice The state machine id of nexus, the chain BRIDGE is native to
     * @return The nexus state machine identifier, "POLKADOT-3367"
     */
    function nexus() public pure returns (bytes memory) {
        return StateMachine.polkadot(NEXUS_PARA_ID);
    }
}
