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


struct BandwidthPurchaseMsg {
    address app;
    uint256 bytesPurchased;
    /// Scaled to 18 decimals so the pallet is decimal-agnostic.
    uint256 amountPaid;
}

contract BandwidthMarket is HyperApp, ERC165 {
    using Bytes for bytes;
    using SafeERC20 for IERC20;

    /// Must equal `pallet-bandwidth`'s `PalletId`.
    bytes public constant PALLET_BANDWIDTH_MODULE_ID = bytes("BWMARKET");

    enum OnAcceptActions {
        SetPricePerByte,
        Withdraw
    }

    address public immutable host_;

    /// Canonical 18-decimal units.
    uint256 public pricePerByte;

    event BandwidthPurchased(
        address indexed app,
        address indexed payer,
        address feeToken,
        uint256 amountPaid,
        uint256 bytesPurchased,
        bytes32 commitment
    );

    event PricePerByteUpdated(uint256 oldPrice, uint256 newPrice);
    event Withdrawn(address indexed token, address indexed beneficiary, uint256 amount);

    error InvalidPurchase();
    error BelowMinimum();
    error UnauthorizedAction();

    constructor(address host__, uint256 pricePerByte_) {
        host_ = host__;
        pricePerByte = pricePerByte_;
    }

    /// @inheritdoc HyperApp
    function host() public view override returns (address) {
        return host_;
    }

    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IApp).interfaceId || super.supportsInterface(interfaceId);
    }

    /// @notice Purchase bandwidth for `app`. `amount` is in the host's
    /// current `feeToken()` decimals.
    function purchase(address app, uint256 amount) external returns (bytes32 commitment) {
        if (amount == 0 || pricePerByte == 0) revert InvalidPurchase();

        address feeToken = IHost(host_).feeToken();
        uint8 dec = IERC20Metadata(feeToken).decimals();

        IERC20(feeToken).safeTransferFrom(msg.sender, address(this), amount);

        // Same stablecoin has different decimals across chains
        // (USDC: 6 on Ethereum, 18 on BSC) — normalise so one
        // `pricePerByte` works everywhere.
        uint256 amountScaled = amount * (10 ** (18 - dec));
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
            feeToken: feeToken,
            amountPaid: amountScaled,
            bytesPurchased: bytesPurchased,
            commitment: commitment
        });
    }

    /// `onlyHost` + source-chain check pin the caller to hyperbridge
    /// governance. `Withdraw` takes an explicit token so balances held
    /// in a stale `feeToken()` remain recoverable after a host swap.
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
            (address token, address beneficiary, uint256 amt) =
                abi.decode(request.body[1:], (address, address, uint256));
            IERC20(token).safeTransfer(beneficiary, amt);
            emit Withdrawn(token, beneficiary, amt);
        } else {
            revert UnauthorizedAction();
        }
    }
}
