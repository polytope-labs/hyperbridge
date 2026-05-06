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
    bytes app;
    uint256 tier;
    /// UTF-8 chain id like `"EVM-8453"` or `"EVM-137"`.
    bytes appChain;
}

contract BandwidthManager is HyperApp, ERC165 {
    using Bytes for bytes;
    using SafeERC20 for IERC20;

    /// Must equal `pallet-bandwidth`'s `PalletId`.
    bytes public constant PALLET_BANDWIDTH_MODULE_ID = bytes("BWMARKET");

    enum OnAcceptActions {
        SetTiers,
        Withdraw
    }

    address public immutable host_;

    /// tier → price in 18-decimal units. Zero = unconfigured.
    mapping(uint256 => uint256) public tierPrice;

    event BandwidthPurchased(
        address indexed payer,
        address feeToken,
        uint256 tier,
        uint256 amountPaid,
        bytes app,
        bytes appChain,
        bytes32 commitment
    );
    event TierSet(uint256 indexed tier, uint256 price18d);
    event Withdrawn(address indexed token, address indexed beneficiary, uint256 amount);

    error InvalidPurchase();
    error UnknownTier();
    /// 18-d tier price doesn't divide cleanly into `feeToken()` decimals.
    error PriceNotRepresentable();
    error UnauthorizedAction();

    constructor(address host__) {
        host_ = host__;
    }

    /// @inheritdoc HyperApp
    function host() public view override returns (address) {
        return host_;
    }

    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IApp).interfaceId || super.supportsInterface(interfaceId);
    }

    function purchase(bytes calldata app, uint256 tier, bytes calldata appChain)
        external
        returns (bytes32 commitment)
    {
        if (app.length == 0 || appChain.length == 0) revert InvalidPurchase();
        uint256 price18d = tierPrice[tier];
        if (price18d == 0) revert UnknownTier();

        address feeToken = IHost(host_).feeToken();
        uint8 dec = IERC20Metadata(feeToken).decimals();
        uint256 scale = 10 ** (18 - dec);
        if (price18d % scale != 0) revert PriceNotRepresentable();
        uint256 amount = price18d / scale;

        IERC20(feeToken).safeTransferFrom(msg.sender, address(this), amount);

        BandwidthPurchaseMsg memory body = BandwidthPurchaseMsg({
            app: app,
            tier: tier,
            appChain: appChain
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
            amountPaid: amount,
            app: app,
            appChain: appChain,
            commitment: commitment
        });
    }


    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        PostRequest calldata request = incoming.request;

        if (!request.source.equals(IHost(host_).hyperbridge())) revert UnauthorizedAction();

        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));
        if (action == OnAcceptActions.SetTiers) {
            (uint256[] memory tiers, uint256[] memory prices) =
                abi.decode(request.body[1:], (uint256[], uint256[]));
            if (tiers.length != prices.length) revert UnauthorizedAction();
            for (uint256 i = 0; i < tiers.length; i++) {
                tierPrice[tiers[i]] = prices[i];
                emit TierSet(tiers[i], prices[i]);
            }
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
