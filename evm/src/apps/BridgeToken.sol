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
import {IncomingPostRequest, PostRequestTimeout} from "@hyperbridge/core/interfaces/IApp.sol";

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

    /// @notice The only relayer whose deliveries mint; zero means nobody
    address internal _relayer;

    /// @notice Emitted when the authorised relayer is replaced
    event RelayerUpdated(address previous, address current);

    /// @notice Thrown when a delivery comes from a relayer other than the authorised one
    error UnauthorizedRelayer();

    /**
     * @notice Deploys the token with nexus already registered as a peer
     * @param initialOwner The address that will own this contract. It can register further peers,
     * pause cross chain operations, change the dispatcher, and set the relayer, so it should be a
     * multisig or a governance controlled account.
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

    /// @notice The relayer authorised to deliver to this token
    function relayer() external view returns (address) {
        return _relayer;
    }

    /**
     * @notice Sets the only relayer whose `onAccept` and `onPostRequestTimeout` deliveries mint
     * @dev Owner-only. The host records a refused delivery as undelivered, so a rejected message can
     * be resubmitted by the authorised relayer.
     * @param newRelayer The relayer to authorise. Zero closes the token to every relayer.
     */
    function setRelayer(address newRelayer) external onlyOwner {
        emit RelayerUpdated({previous: _relayer, current: newRelayer});
        _relayer = newRelayer;
    }

    /// @dev Gated on the relayer before the base token mints. See `_checkRelayer`.
    function onAccept(IncomingPostRequest calldata incoming) public override onlyHost {
        _checkRelayer(incoming.relayer);
        super.onAccept(incoming);
    }

    /// @dev Gated on the relayer before the base token refunds. See `_checkRelayer`.
    function onPostRequestTimeout(PostRequestTimeout memory incoming) public override onlyHost {
        _checkRelayer(incoming.relayer);
        super.onPostRequestTimeout(incoming);
    }

    /**
     * @dev Fails closed: with no relayer set nobody may deliver. The supply of this token is backed
     * by the nexus escrow, so it must not mint on the strength of a consensus proof alone. The
     * handler always forwards a real `msg.sender`, so zero never matches.
     */
    function _checkRelayer(address incomingRelayer) private view {
        if (incomingRelayer != _relayer) revert UnauthorizedRelayer();
    }
}
