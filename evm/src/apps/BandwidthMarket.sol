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

/// Body of a bandwidth-purchase message dispatched to `pallet-bandwidth`.
/// Field layout must stay in lockstep with the ABI struct decoded on the
/// substrate side.
struct BandwidthPurchaseMsg {
    address app;
    uint256 bytesPurchased;
    /// Amount paid scaled to canonical 18 decimals so the pallet does not
    /// need per-chain decimal knowledge.
    uint256 amountPaid;
}

/// @notice Per-host contract that sells prepaid bandwidth on hyperbridge.
/// Pulls a stablecoin from the buyer, derives bytes from `pricePerByte`,
/// and dispatches a `PostRequest` to `pallet-bandwidth`. The pallet
/// credits the `(source_chain, app)` balance; the hyperbridge router
/// decrements it as the app's traffic flows.
///
/// `EvmHost` is intentionally unmodified — `pricePerByte` is mutated only
/// via `onAccept`, mirroring `HostManager.onAccept` so updates flow
/// through the existing cross-chain governance pipeline.
contract BandwidthMarket is HyperApp, ERC165 {
    using Bytes for bytes;
    using SafeERC20 for IERC20;

    /// Raw bytes of `ModuleId::Pallet(PalletId(*b"BWMARKET"))` — used as
    /// the `to` field of dispatched purchase messages.
    bytes public constant PALLET_BANDWIDTH_MODULE_ID = bytes("BWMARKET");

    enum OnAcceptActions {
        SetPricePerByte,
        Withdraw
    }

    address public immutable host_;
    /// Distinct from `IHost.feeToken()` because the host's fee token is
    /// swapped to the NoOp ERC-20 during the bandwidth-mode cutover.
    address public immutable stablecoin;
    /// Snapshot of `stablecoin.decimals()` at deploy time, capped at 18.
    uint8 public immutable tokenDecimals;

    /// Price per byte in canonical 18-decimal units.
    uint256 public pricePerByte;

    event BandwidthPurchased(
        address indexed app,
        address indexed payer,
        uint256 amountPaid,
        uint256 bytesPurchased,
        bytes32 commitment
    );

    event PricePerByteUpdated(uint256 oldPrice, uint256 newPrice);
    event Withdrawn(address indexed beneficiary, uint256 amount);

    error InvalidPurchase();
    error BelowMinimum();
    error UnsupportedDecimals();
    error UnauthorizedAction();

    constructor(address host__, address stablecoin_, uint256 pricePerByte_) {
        uint8 dec = IERC20Metadata(stablecoin_).decimals();
        if (dec > 18) revert UnsupportedDecimals();

        host_ = host__;
        stablecoin = stablecoin_;
        tokenDecimals = dec;
        pricePerByte = pricePerByte_;
    }

    /// @inheritdoc HyperApp
    function host() public view override returns (address) {
        return host_;
    }

    /// @dev See {IERC165-supportsInterface}.
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IApp).interfaceId || super.supportsInterface(interfaceId);
    }

    /// @notice Purchase bandwidth on hyperbridge for `app`. `amount` is in
    /// the stablecoin's native decimals; the buyer can be a paymaster on
    /// behalf of another contract.
    function purchase(address app, uint256 amount) external returns (bytes32 commitment) {
        if (amount == 0 || pricePerByte == 0) revert InvalidPurchase();

        IERC20(stablecoin).safeTransferFrom(msg.sender, address(this), amount);

        // Scale to 18 decimals so one `pricePerByte` value works across
        // chains where the same stablecoin has different decimals
        // (USDC: 6 on Ethereum, 18 on BSC). Constructor bounds dec ≤ 18.
        uint256 amountScaled = amount * (10 ** (18 - tokenDecimals));
        uint256 bytesPurchased = amountScaled / pricePerByte;
        if (bytesPurchased == 0) revert BelowMinimum();

        BandwidthPurchaseMsg memory body = BandwidthPurchaseMsg({
            app: app,
            bytesPurchased: bytesPurchased,
            amountPaid: amountScaled
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
            app: app,
            payer: msg.sender,
            amountPaid: amountScaled,
            bytesPurchased: bytesPurchased,
            commitment: commitment
        });
    }

    /// Defence-in-depth: `onlyHost` restricts the caller to the local
    /// `EvmHost`, and the source-chain check restricts the origin to
    /// hyperbridge governance. Mirrors `HostManager.onAccept`.
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        PostRequest calldata request = incoming.request;

        if (!request.source.equals(IHost(host_).hyperbridge())) revert UnauthorizedAction();

        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.SetPricePerByte) {
            uint256 newPrice = abi.decode(request.body[1:], (uint256));
            uint256 oldPrice = pricePerByte;
            pricePerByte = newPrice;
            emit PricePerByteUpdated(oldPrice, newPrice);
        } else if (action == OnAcceptActions.Withdraw) {
            (address beneficiary, uint256 amt) = abi.decode(request.body[1:], (address, uint256));
            IERC20(stablecoin).safeTransfer(beneficiary, amt);
            emit Withdrawn(beneficiary, amt);
        } else {
            revert UnauthorizedAction();
        }
    }
}
