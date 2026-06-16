// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import {HyperFungibleToken} from "@hyperbridge/core/apps/HyperFungibleToken.sol";
import {BandwidthPurchaseMsg, Tier, Withdrawal} from "../../src/apps/BandwidthManager.sol";
import {Test} from "forge-std/Test.sol";

/// @dev Exposes encode/decode helpers for cross-language testing of app
/// payloads (HFT Message, BandwidthPurchaseMsg, Tier[], Withdrawal).
/// Each `encode*` returns what production Solidity calls (`abi.encode(x)`)
/// and each `decode*` is what production Solidity expects on the
/// receiving side (`abi.decode(data, (T))`).
contract AbiAppsCodec {
    // ── HyperFungibleToken Message ────────────────────────────────────
    // Production: `sdk/packages/core/contracts/apps/HyperFungibleToken.sol:242,293,311`
    // `Message` is nested inside the `HyperFungibleToken` contract.

    function encodeHftMessage(HyperFungibleToken.Message memory m)
        external
        pure
        returns (bytes memory)
    {
        return abi.encode(m);
    }

    function decodeHftMessage(bytes memory data)
        external
        pure
        returns (HyperFungibleToken.Message memory)
    {
        return abi.decode(data, (HyperFungibleToken.Message));
    }

    // ── BandwidthPurchaseMsg ─────────────────────────────────────────
    // Production: `evm/src/apps/BandwidthManager.sol:177` (outbound to pallet)

    function encodeBandwidthPurchase(BandwidthPurchaseMsg memory m)
        external
        pure
        returns (bytes memory)
    {
        return abi.encode(m);
    }

    function decodeBandwidthPurchase(bytes memory data)
        external
        pure
        returns (BandwidthPurchaseMsg memory)
    {
        return abi.decode(data, (BandwidthPurchaseMsg));
    }

    // ── Tier[] (SetTiers governance payload) ─────────────────────────
    // Production: `evm/src/apps/BandwidthManager.sol:209` (inbound from pallet)

    function encodeTiers(Tier[] memory tiers) external pure returns (bytes memory) {
        return abi.encode(tiers);
    }

    function decodeTiers(bytes memory data) external pure returns (Tier[] memory) {
        return abi.decode(data, (Tier[]));
    }

    // ── Withdrawal (Withdraw governance payload) ─────────────────────
    // Production: `evm/src/apps/BandwidthManager.sol:215` (inbound from pallet)

    function encodeWithdrawal(Withdrawal memory w) external pure returns (bytes memory) {
        return abi.encode(w);
    }

    function decodeWithdrawal(bytes memory data) external pure returns (Withdrawal memory) {
        return abi.decode(data, (Withdrawal));
    }
}
