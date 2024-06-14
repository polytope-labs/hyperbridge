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
pragma solidity 0.8.17;

import {IDispatcher, DispatchPost} from "ismp/IDispatcher.sol";
import {IIsmpHost} from "ismp/IIsmpHost.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {BaseIsmpModule, PostRequest, IncomingPostRequest} from "ismp/IIsmpModule.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {IERC6160Ext20} from "ERC6160/interfaces/IERC6160Ext20.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin/token/ERC20/utils/SafeERC20.sol";
import {ICallDispatcher, CallDispatcherParams} from "./CallDispatcher.sol";
import {IUniswapV2Router} from "../interfaces/IUniswapV2Router.sol";

struct TeleportParams {
    // amount to be sent
    uint256 amount;
    // Relayer fee
    uint256 fee;
    // The token identifier
    bytes32 assetId;
    // Redeem Erc20 on the destination?
    bool redeem;
    // recipient address
    bytes32 to;
    // The Erc20 token to be used to swap for a fee
    address feeToken;
    // recipient state machine
    bytes dest;
    // timeout in seconds
    uint64 timeout;
    // destination contract call data
    bytes data;
    // calculated amountInMax:
    // used if selected fee token is not expected fee token
    uint256 amountInMax;
}

struct Body {
    // amount to be sent
    uint256 amount;
    // The token identifier
    bytes32 assetId;
    // flag to redeem the erc20 asset on the destination
    bool redeem;
    // sender address
    bytes32 from;
    // recipient address
    bytes32 to;
}

struct BodyWithCall {
    // amount to be sent
    uint256 amount;
    // The token identifier
    bytes32 assetId;
    // flag to redeem the erc20 asset on the destination
    bool redeem;
    // sender address
    bytes32 from;
    // recipient address
    bytes32 to;
    // calldata to be sent to the destination contract along aside with the asset
    bytes data;
}

struct TokenGatewayParamsExt {
    // Initial params for TokenGateway
    TokenGatewayParams params;
    // List of initial assets
    SetAsset[] assets;
}

struct Asset {
    // ERC20 token contract address for the asset
    address erc20;
    // ERC6160 token contract address for the asset
    address erc6160;
    // Asset's identifier
    bytes32 identifier;
    // Associated fees for this asset
    AssetFees fees;
}

enum OnAcceptActions {
    // Incoming asset from a chain
    IncomingAsset,
    // Governance action to update protocol parameters
    GovernanceAction,
    // Either register a new asset or update an existing asset
    SetAssets,
    // Remove an asset from the registry
    DeregisterAsset,
    // Change the admin of an asset
    ChangeAssetAdmin
}

struct SetAsset {
    // ERC20 token contract address for the asset
    address erc20;
    // ERC6160 token contract address for the asset
    address erc6160;
    // Asset's name
    string name;
    // Asset's symbol
    string symbol;
    // Associated fees for this asset
    AssetFees fees;
}

struct ChangeAssetAdmin {
    // Address of the asset
    address erc6160;
    // The address of the new admin
    address newAdmin;
}

struct AssetFees {
    // Fee percentage paid to relayers for this asset
    uint256 relayerFeePercentage;
    // Fee percentage paid to the protocol for this asset
    uint256 protocolFeePercentage;
}

// Abi-encoded size of Body struct
uint256 constant BODY_BYTES_SIZE = 161;

struct TokenGatewayParams {
    // address of the IsmpHost contract on this chain
    address host;
    // local uniswap router
    address uniswapV2;
    // dispatcher for delegating external calls
    address dispatcher;
    // Wrapped ERC20 contract address for native token
    address erc20NativeToken;
}

// The TokenGateway allows users send either ERC20 or ERC6160 tokens
// using Hyperbridge as a message-passing layer.
contract TokenGateway is BaseIsmpModule {
    using Bytes for bytes;

    // TokenGateway protocol parameters
    TokenGatewayParams private _params;

    // admin account
    address private _admin;

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;
    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;
    // mapping of token identifier to it's associated fees
    mapping(bytes32 => AssetFees) private _fees;

    // Relayer provided some liquidity
    event LiquidityProvided(address indexed relayer, uint256 amount, bytes32 indexed assetId);
    // User has received some assets
    event AssetReceived(
        bytes indexed source, uint256 nonce, address indexed beneficiary, uint256 amount, bytes32 indexed assetId
    );
    // User has sent some assets
    event AssetTeleported(
        address from, bytes32 to, uint256 amount, bytes32 assetId, bool redeem, bytes32 requestCommitment
    );
    // User assets could not be delivered and have been refunded.
    event AssetRefunded(
        address beneficiary, uint256 amount, bytes32 indexed assetId, bytes dest, uint256 indexed nonce
    );

    // Action is unauthorized
    error UnauthorizedAction();
    error ZeroAddress();
    error InvalidAmount();
    error InvalidFeeToken();
    error UnknownToken();
    error InconsistentState();

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != _params.host) revert UnauthorizedAction();
        _;
    }

    // restricts call to `admin`
    modifier onlyAdmin() {
        if (msg.sender != _admin) revert UnauthorizedAction();
        _;
    }

    constructor(address admin) {
        _admin = admin;
    }

    // initialize required parameters
    function init(TokenGatewayParamsExt memory teleportParams) public onlyAdmin {
        _params = teleportParams.params;
        setAssets(teleportParams.assets);

        // admin can only call this once
        _admin = address(0);
    }

    // Read the protocol parameters
    function params() external view returns (TokenGatewayParams memory) {
        return _params;
    }

    // Fetch the address for an ERC20 asset
    function erc20(bytes32 assetId) external view returns (address) {
        return _erc20s[assetId];
    }

    // Fetch the address for an ERC6160 asset
    function erc6160(bytes32 assetId) external view returns (address) {
        return _erc6160s[assetId];
    }

    // Fetch the fees for a given asset id
    function fees(bytes32 assetId) external view returns (AssetFees memory) {
        return _fees[assetId];
    }

    // Teleport a given asset to the destination chain. Allows users to pay
    // the Hyperbridge fees in any token or the native asset.
    function teleport(TeleportParams memory teleportParams) public payable {
        if (teleportParams.to == bytes32(0)) {
            revert ZeroAddress();
        }
        if (teleportParams.amount == 0) {
            revert InvalidAmount();
        }

        address from = msg.sender;
        bytes32 fromBytes32 = addressToBytes32(msg.sender);
        address _erc20 = _erc20s[teleportParams.assetId];
        address _erc6160 = _erc6160s[teleportParams.assetId];
        address feeToken = IIsmpHost(_params.host).feeToken();

        bytes memory data = teleportParams.data.length > 0
            ? abi.encode(
                BodyWithCall({
                    from: fromBytes32,
                    to: teleportParams.to,
                    amount: teleportParams.amount,
                    assetId: teleportParams.assetId,
                    redeem: teleportParams.redeem,
                    data: teleportParams.data
                })
            )
            : abi.encode(
                Body({
                    from: fromBytes32,
                    to: teleportParams.to,
                    amount: teleportParams.amount,
                    assetId: teleportParams.assetId,
                    redeem: teleportParams.redeem
                })
            );
        data = bytes.concat(hex"00", data); // add enum variant for body

        if (_erc20 != address(0) && !teleportParams.redeem) {
            SafeERC20.safeTransferFrom(IERC20(_erc20), from, address(this), teleportParams.amount);
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).burn(from, teleportParams.amount);
        } else {
            revert UnknownToken();
        }

        uint256 fee = (IIsmpHost(_params.host).perByteFee() * data.length) + teleportParams.fee;
        // only swap if the feeToken is not the token intended for fee
        if (feeToken != teleportParams.feeToken) {
            if (msg.value != 0) {
                // user has opted to pay with the native asset, wrap it in ERC20
                (bool sent,) = _params.erc20NativeToken.call{value: msg.value}("");
                if (!sent) revert InconsistentState();
                teleportParams.feeToken = _params.erc20NativeToken;
                teleportParams.amountInMax = msg.value;
            } else {
                SafeERC20.safeTransferFrom(
                    IERC20(teleportParams.feeToken), from, address(this), teleportParams.amountInMax
                );
            }

            SafeERC20.safeIncreaseAllowance(
                IERC20(teleportParams.feeToken), _params.uniswapV2, teleportParams.amountInMax
            );

            address[] memory path = new address[](2);
            path[0] = teleportParams.feeToken;
            path[1] = feeToken;

            IUniswapV2Router(_params.uniswapV2).swapTokensForExactTokens(
                fee, teleportParams.amountInMax, path, address(this), block.timestamp
            );
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken), from, address(this), fee);
        }

        // approve the host with the exact amount
        SafeERC20.safeIncreaseAllowance(IERC20(feeToken), _params.host, fee);
        DispatchPost memory request = DispatchPost({
            dest: teleportParams.dest,
            to: abi.encodePacked(address(this)),
            body: data,
            timeout: teleportParams.timeout,
            fee: teleportParams.fee,
            payer: msg.sender
        });
        bytes32 commitment = IDispatcher(_params.host).dispatch(request);

        emit AssetTeleported({
            from: from,
            to: teleportParams.to,
            assetId: teleportParams.assetId,
            amount: teleportParams.amount,
            redeem: teleportParams.redeem,
            requestCommitment: commitment
        });
    }

    function onAccept(IncomingPostRequest calldata incoming) external override onlyIsmpHost {
        OnAcceptActions action = OnAcceptActions(uint8(incoming.request.body[0]));

        if (action == OnAcceptActions.IncomingAsset) {
            if (incoming.request.body.length > BODY_BYTES_SIZE) {
                handleIncomingAssetWithCall(incoming);
            } else {
                handleIncomingAssetWithoutCall(incoming);
            }
        } else if (action == OnAcceptActions.GovernanceAction) {
            handleGovernance(incoming.request);
        } else if (action == OnAcceptActions.SetAssets) {
            handleSetAssets(incoming.request);
        } else if (action == OnAcceptActions.DeregisterAsset) {
            deregisterAssets(incoming.request);
        } else if (action == OnAcceptActions.ChangeAssetAdmin) {
            changeAssetAdmin(incoming.request);
        }
    }

    // Triggered when a previously sent out request is confirmed to be timed-out by the IsmpHost.
    // This means the funds could not be sent, we simply refund the user's assets here.
    function onPostRequestTimeout(PostRequest calldata request) external override onlyIsmpHost {
        Body memory body;
        if (request.body.length > BODY_BYTES_SIZE) {
            BodyWithCall memory bodyWithCall = abi.decode(request.body[1:], (BodyWithCall));
            body = Body({
                amount: bodyWithCall.amount,
                assetId: bodyWithCall.assetId,
                redeem: bodyWithCall.redeem,
                from: bodyWithCall.from,
                to: bodyWithCall.to
            });
        } else {
            body = abi.decode(request.body[1:], (Body));
        }

        address _erc20 = _erc20s[body.assetId];
        address _erc6160 = _erc6160s[body.assetId];
        address from = bytes32ToAddress(body.from);

        if (_erc20 != address(0) && !body.redeem) {
            SafeERC20.safeTransfer(IERC20(_erc20), from, body.amount);
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(from, body.amount);
        } else {
            revert InconsistentState();
        }

        emit AssetRefunded({
            beneficiary: from,
            amount: body.amount,
            assetId: body.assetId,
            dest: request.dest,
            nonce: request.nonce
        });
    }

    function handleIncomingAssetWithoutCall(IncomingPostRequest calldata incoming) private {
        PostRequest calldata request = incoming.request;
        // TokenGateway only accepts incoming assets from it's instances on other chains.
        if (!request.from.equals(abi.encodePacked(address(this)))) revert UnauthorizedAction();

        Body memory body = abi.decode(request.body[1:], (Body));
        address to = bytes32ToAddress(body.to);
        handleIncomingAsset(body.assetId, body.redeem, body.amount, to, incoming.relayer);

        emit AssetReceived({
            source: request.source,
            nonce: request.nonce,
            beneficiary: to,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    function handleIncomingAssetWithCall(IncomingPostRequest calldata incoming) private {
        PostRequest calldata request = incoming.request;
        // TokenGateway only accepts incoming assets from it's instances on other chains.
        if (!request.from.equals(abi.encodePacked(address(this)))) revert UnauthorizedAction();

        BodyWithCall memory body = abi.decode(request.body[1:], (BodyWithCall));
        address to = bytes32ToAddress(body.to);
        handleIncomingAsset(body.assetId, body.redeem, body.amount, to, incoming.relayer);

        // dispatching low level call
        CallDispatcherParams memory dispatcherParams = abi.decode(body.data, (CallDispatcherParams));
        ICallDispatcher(_params.dispatcher).dispatch(dispatcherParams);

        emit AssetReceived({
            source: request.source,
            nonce: request.nonce,
            beneficiary: to,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    function handleIncomingAsset(bytes32 assetId, bool redeem, uint256 amount, address to, address relayer) private {
        address _erc20 = _erc20s[assetId];
        address _erc6160 = _erc6160s[assetId];

        if (_erc20 != address(0) && redeem) {
            // a relayer/user is redeeming the native asset
            uint256 transferredAmount = amount - protocolFee(assetId, amount);
            SafeERC20.safeTransfer(IERC20(_erc20), to, transferredAmount);
        } else if (_erc20 != address(0) && _erc6160 != address(0) && !redeem) {
            // user is swapping, relayers should double as liquidity providers.
            uint256 toTransfer = amount - relayerLiquidityFee(assetId, amount);
            SafeERC20.safeTransferFrom(IERC20(_erc20), relayer, to, toTransfer);
            // hand the relayer the erc6160, so they can redeem on the source chain
            IERC6160Ext20(_erc6160).mint(relayer, amount);
            emit LiquidityProvided({relayer: relayer, amount: toTransfer, assetId: assetId});
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(to, amount);
        } else {
            revert UnknownToken();
        }
    }

    function handleGovernance(PostRequest calldata request) private {
        // only hyperbridge can do this
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        _params = abi.decode(request.body[1:], (TokenGatewayParams));
    }

    function handleSetAssets(PostRequest calldata request) private {
        // only hyperbridge can do this
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        SetAsset[] memory assets = abi.decode(request.body[1:], (SetAsset[]));
        setAssets(assets);
    }

    function deregisterAssets(PostRequest calldata request) private {
        // only hyperbridge can do this
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        bytes32[] memory identifiers = abi.decode(request.body[1:], (bytes32[]));
        uint256 length = identifiers.length;
        for (uint256 i = 0; i < length; ++i) {
            delete _erc20s[identifiers[i]];
            delete _erc6160s[identifiers[i]];
            delete _fees[identifiers[i]];
        }
    }

    function changeAssetAdmin(PostRequest calldata request) private {
        // only hyperbridge can do this
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        ChangeAssetAdmin[] memory assets = abi.decode(request.body[1:], (ChangeAssetAdmin[]));
        uint256 length = assets.length;
        for (uint256 i = 0; i < length; ++i) {
            ChangeAssetAdmin memory asset = assets[i];
            if (asset.newAdmin == address(0)) revert ZeroAddress();
            IERC6160Ext20(asset.erc6160).changeAdmin(asset.newAdmin);
        }
    }

    function setAssets(SetAsset[] memory assets) private {
        uint256 length = assets.length;
        for (uint256 i = 0; i < length; ++i) {
            SetAsset memory asset = assets[i];
            bytes32 identifier = keccak256(bytes(asset.symbol));
            if (asset.erc6160 == address(0)) {
                ERC6160Ext20 erc6160Asset = new ERC6160Ext20{salt: identifier}(address(this), asset.name, asset.symbol);
                asset.erc6160 = address(erc6160Asset);
            }
            _erc20s[identifier] = asset.erc20;
            _erc6160s[identifier] = asset.erc6160;
            _fees[identifier] = asset.fees;
        }
    }

    function relayerLiquidityFee(bytes32 assetId, uint256 amount) private view returns (uint256 liquidityFee) {
        liquidityFee = (amount * _fees[assetId].relayerFeePercentage) / 100_000;
    }

    function protocolFee(bytes32 assetId, uint256 amount) private view returns (uint256 redeemFee) {
        redeemFee = (amount * _fees[assetId].protocolFeePercentage) / 100_000;
    }

    function addressToBytes32(address _address) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }

    function bytes32ToAddress(bytes32 _bytes) internal pure returns (address) {
        return address(uint160(uint256(_bytes)));
    }
}
