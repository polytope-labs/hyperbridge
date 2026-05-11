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

import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {DispatchPost, IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {IncomingPostRequest, IApp} from "@hyperbridge/core/interfaces/IApp.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";


/// Wire payload dispatched by `purchase()` to `pallet-bandwidth`. The
/// pallet credits a tier-bucket on `chain` for `app`, scaled by `months`.
struct BandwidthPurchaseMsg {
    /// Recipient app whose bandwidth is being topped up.
    bytes app;
    /// Tier discriminant (matches `pallet_bandwidth::TierIndex`).
    uint256 tier;
    /// Number of tier-windows to credit. Bytes and duration both scale.
    uint256 months;
    /// UTF-8 chain id like `"EVM-8453"` or `"EVM-137"`.
    bytes chain;
}

/// One row of a `SetTiers` governance batch.
struct Tier {
    /// Tier discriminant (matches `pallet_bandwidth::TierIndex`).
    uint256 tier;
    /// Price in 18-decimal units; scaled at purchase time to fee-token decimals.
    uint256 price;
}

/// Payload of a `Withdraw` governance message — recovers `amount` of
/// `token` to `beneficiary`. `token` is named explicitly so stale
/// fee-token balances after a host-side swap can still be drained.
struct Withdrawal {
    address token;
    address beneficiary;
    uint256 amount;
}

/// @title BandwidthManager
/// @notice Per-chain prepaid bandwidth storefront. Buyers call
/// `purchase()` to debit a fee-token and dispatch a credit message to
/// `pallet-bandwidth` on hyperbridge; tier prices and treasury
/// withdrawals are governed exclusively by the pallet via `onAccept`.
contract BandwidthManager is HyperApp, ERC165 {
    using Bytes for bytes;
    using SafeERC20 for IERC20;

    /// Must equal `pallet-bandwidth`'s `PalletId`. The pallet enforces
    /// this on inbound messages, so changing it on either side breaks
    /// the round-trip.
    bytes public constant PALLET_BANDWIDTH_MODULE_ID = bytes("BWMARKET");

    /// Discriminants for the first byte of an `onAccept` body. Order
    /// must match `pallet_bandwidth::lib.rs::ACTION_*`.
    enum OnAcceptActions {
        SetTiers,
        Withdraw
    }

    address public immutable host_;

    /// tier → price in 18-decimal units. Zero = unconfigured (purchases
    /// against an unconfigured tier revert with `UnknownTier`).
    mapping(uint256 => uint256) public tierPrice;

    /// Emitted on a successful `purchase()`. `commitment` is the
    /// hyperbridge dispatch commitment so callers can correlate with
    /// the pallet-side credit event.
    event BandwidthPurchased(
        address indexed payer,
        address feeToken,
        uint256 tier,
        uint256 months,
        uint256 amountPaid,
        bytes app,
        bytes chain,
        bytes32 commitment
    );
    /// Emitted once per tier in a `SetTiers` governance batch.
    event TierSet(uint256 indexed tier, uint256 price18d);
    /// Emitted by a `Withdraw` governance message after the transfer succeeds.
    event Withdrawn(address indexed token, address indexed beneficiary, uint256 amount);

    /// `app`/`chain` empty, or `months == 0`.
    error InvalidPurchase();
    /// Tier price not configured (`tierPrice[tier] == 0`).
    error UnknownTier();
    /// 18-d tier price doesn't divide cleanly into `feeToken()` decimals.
    error PriceNotRepresentable();
    /// `onAccept` body came from a non-hyperbridge source, or the
    /// action discriminant is out of range.
    error UnauthorizedAction();

    constructor(address host__) {
        host_ = host__;
    }

    /// @inheritdoc HyperApp
    function host() public view override returns (address) {
        return host_;
    }

    /// @inheritdoc ERC165
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IApp).interfaceId || super.supportsInterface(interfaceId);
    }

    /// @notice Pay for `months` of `tier` bandwidth on `chain` for `app`.
    /// @dev Pulls the scaled tier price from `msg.sender` in the host's
    /// fee token, then dispatches a `BandwidthPurchaseMsg` to
    /// `pallet-bandwidth` on hyperbridge. The pallet credits an
    /// `(chain, app)` bucket bounded by tier `bytes` × `months`.
    /// @param app Recipient app address (usually 20-byte EVM, packed as bytes).
    /// @param tier Tier discriminant; must be configured via `SetTiers`.
    /// @param months Number of tier-windows to credit; must be > 0.
    /// @param chain UTF-8 chain id (e.g. `"EVM-8453"`) of the credit chain.
    /// @return commitment Hyperbridge dispatch commitment for tracking.
    function purchase(bytes calldata app, uint256 tier, uint256 months, bytes calldata chain)
        external
        returns (bytes32 commitment)
    {
        if (app.length == 0 || chain.length == 0 || months == 0) revert InvalidPurchase();
        uint256 price18d = tierPrice[tier];
        if (price18d == 0) revert UnknownTier();

        uint256 total18d = price18d * months;
        address feeToken = IHost(host_).feeToken();
        uint8 dec = IERC20Metadata(feeToken).decimals();
        uint256 scale = 10 ** (18 - dec);
        if (total18d % scale != 0) revert PriceNotRepresentable();
        uint256 amount = total18d / scale;

        IERC20(feeToken).safeTransferFrom(msg.sender, address(this), amount);

        BandwidthPurchaseMsg memory body = BandwidthPurchaseMsg({
            app: app,
            tier: tier,
            months: months,
            chain: chain
        });

        commitment = IDispatcher(host_).dispatch(
            DispatchPost({
                dest: IHost(host_).hyperbridge(),
                to: PALLET_BANDWIDTH_MODULE_ID,
                body: abi.encode(body),
                timeout: 0,
                fee: 0,
                payer: address(this)
            })
        );

        emit BandwidthPurchased({
            payer: msg.sender,
            feeToken: feeToken,
            tier: tier,
            months: months,
            amountPaid: amount,
            app: app,
            chain: chain,
            commitment: commitment
        });
    }


    /// @notice Inbound governance from `pallet-bandwidth`. The first
    /// body byte selects `OnAcceptActions`; the remainder is the
    /// action's ABI-encoded payload.
    /// @dev Only the configured host may invoke (`onlyHost`); the
    /// request's `source` must additionally equal hyperbridge.
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        PostRequest calldata request = incoming.request;

        if (!request.source.equals(IHost(host_).hyperbridge())) revert UnauthorizedAction();

        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.SetTiers) {
            Tier[] memory updates = abi.decode(request.body[1:], (Tier[]));
            for (uint256 i = 0; i < updates.length; i++) {
                tierPrice[updates[i].tier] = updates[i].price;
                emit TierSet(updates[i].tier, updates[i].price);
            }
        } else if (action == OnAcceptActions.Withdraw) {
            Withdrawal memory w = abi.decode(request.body[1:], (Withdrawal));
            IERC20(w.token).safeTransfer(w.beneficiary, w.amount);
            emit Withdrawn(w.token, w.beneficiary, w.amount);
        } else {
            revert UnauthorizedAction();
        }
    }
}
