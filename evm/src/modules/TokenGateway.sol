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

import {DispatchPost} from "ismp/IDispatcher.sol";
import {IIsmpHost} from "ismp/IIsmpHost.sol";
import {Message} from "ismp/Message.sol";
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
    // Maximum amount to pay for liquidity fees
    uint256 maxFee;
    // Relayer fee
    uint256 relayerFee;
    // The token identifier
    bytes32 assetId;
    // Redeem Erc20 on the destination?
    bool redeem;
    // recipient address
    bytes32 to;
    // The ERC20 token to be used to swap for a fee
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
    // Amount of the asset to be sent
    uint256 amount;
    // Maximum amount to pay for liquidity fees
    uint256 maxFee;
    // The asset identifier
    bytes32 assetId;
    // Flag to redeem the erc20 asset on the destination
    bool redeem;
    // Sender address
    bytes32 from;
    // Recipient address
    bytes32 to;
}

struct BodyWithCall {
    // Amount of the asset to be sent
    uint256 amount;
    // Maximum amount to pay for liquidity fees
    uint256 maxFee;
    // The asset identifier
    bytes32 assetId;
    // Flag to redeem the erc20 asset on the destination
    bool redeem;
    // Sender address
    bytes32 from;
    // Recipient address
    bytes32 to;
    // Calldata to be passed to the asset destination
    bytes data;
}

struct TokenGatewayParamsExt {
    // Initial params for TokenGateway
    TokenGatewayParams params;
    // List of initial assets
    AssetMetadata[] assets;
}

struct Asset {
    // ERC20 token contract address for the asset
    address erc20;
    // ERC6160 token contract address for the asset
    address erc6160;
    // Asset's identifier
    bytes32 identifier;
}

struct ContractInstance {
    // The state machine identifier for this chain
    bytes chain;
    // The token gateway contract address on this chain
    address moduleId;
}

enum OnAcceptActions {
    // Incoming asset from a chain
    IncomingAsset,
    // Governance action to update protocol parameters
    GovernanceAction,
    // Request from hyperbridge to create a new asset
    CreateAsset,
    // Remove an asset from the registry
    DeregisterAsset,
    // Change the admin of an asset
    ChangeAssetAdmin,
    // Add a new pre-approved address
    NewContractInstance
}

struct AssetMetadata {
    // ERC20 token contract address for the asset
    address erc20;
    // ERC6160 token contract address for the asset
    address erc6160;
    // Asset's name
    string name;
    // Asset's symbol
    string symbol;
    // The initial supply of asset
    uint256 initialSupply;
    // Initial beneficiary of the total supply
    address beneficiary;
}

struct ChangeAssetAdmin {
    // Address of the asset
    bytes32 assetId;
    // The address of the new admin
    address newAdmin;
}

struct DeregsiterAsset {
    // List of assets to deregister
    bytes32[] assetIds;
}

// Abi-encoded size of Body struct
uint256 constant BODY_BYTES_SIZE = 193;

// Params for the TokenGateway contract
struct TokenGatewayParams {
    // address of the IsmpHost contract on this chain
    address host;
    // local uniswap router
    address uniswapV2;
    // dispatcher for delegating external calls
    address dispatcher;
}

struct LiquidityBid {
    // Bidder in question
    address bidder;
    // Proposed fee
    uint256 fee;
}

/**
 * @title The TokenGateway.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Allows users send either ERC20 or ERC6160 tokens using Hyperbridge as a message-passing layer.
 *
 * @dev If ERC20 tokens are sent then fillers step in to provide the ERC20 token on the destination chain.
 * Otherwise if ERC6160 tokens are sent, then it simply performs a burn-and-mint.
 */
contract TokenGateway is BaseIsmpModule {
    using Bytes for bytes;
    using Message for PostRequest;

    // TokenGateway protocol parameters
    TokenGatewayParams private _params;

    // admin account
    address private _admin;

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;

    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;

    // mapping of a request commitment to a corresponding bid
    mapping(bytes32 => LiquidityBid) private _bids;

    // mapping of keccak256(source chain) to the token gateway contract address
    mapping(bytes32 => address) private _instances;

    // A filler has just placed a bid to fulfil some request
    event BidPlaced(
        // The associated request commitment
        bytes32 commitment,
        // The liquidity fee for the bid
        uint256 bid,
        // The assetId for the bid
        bytes32 indexed assetId,
        // The bidder's address
        address indexed bidder
    );

    // The request associated with a bid has timed out and the bid refunded
    event BidRefunded(
        // The associated request commitment
        bytes32 commitment,
        // The assetId for the bid
        bytes32 indexed assetId,
        // The bidder's address
        address indexed bidder
    );

    // Filler fulfilled some liquidity request
    event RequestFulfilled(
        // The amount that was provided to the user
        uint256 amount,
        // The bidder's address
        address indexed bidder,
        // The provided assetId
        bytes32 indexed assetId
    );

    // User has received some assets
    event AssetReceived(
        // The amount that was provided to the user
        uint256 amount,
        // The associated request commitment
        bytes32 commitment,
        // The source of the funds
        bytes32 indexed from,
        // The beneficiary of the funds
        address indexed beneficiary,
        // The provided assetId
        bytes32 indexed assetId
    );

    // User has sent some assets
    event AssetTeleported(
        // The beneficiary of the funds
        bytes32 to,
        // The amount that was requested to be sent
        uint256 amount,
        // The associated request commitment
        bytes32 commitment,
        // The source of the funds
        address indexed from,
        // The provided assetId
        bytes32 indexed assetId,
        // Flag to redeem funds from the TokenGateway
        bool redeem
    );

    // User assets could not be delivered and have been refunded.
    event AssetRefunded(
        // The amount that was requested to be sent
        uint256 amount,
        // The associated request commitment
        bytes32 commitment,
        // The beneficiary of the funds
        address indexed beneficiary,
        // The provided assetId
        bytes32 indexed assetId
    );

    // A new asset has been registered
    event AssetRegistered(
        // ERC20 token contract address for the asset
        address erc20,
        // ERC6160 token contract address for the asset
        address erc6160,
        // Asset's name
        string name,
        // Asset's symbol
        string symbol,
        // Registered asset identifier
        bytes32 assetId,
        // The initial supply of asset
        uint256 initialSupply,
        // Initial beneficiary of the total supply
        address beneficiary
    );

    // A new contract instance has been registered
    event NewContractInstance(
        // The chain for this new contract instance
        string chain,
        // The address for token gateway on this chain
        address moduleId
    );

    // Contract parameters have been updated by Hyperbridge governance
    event ParamsUpdated(
        // The old token gateway params
        TokenGatewayParams oldParams,
        // The new token gateway params
        TokenGatewayParams newParams
    );

    // An asset has been deregistered
    event AssetRemoved(bytes32 assetId);

    // An asset owner has requested to change the admin of their asset
    event AssetAdminChanged(
        // The ERC6160 token contract address
        address asset,
        // The new admin
        address newAdmin
    );

    // Action is unauthorized
    error UnauthorizedAction();

    // Request is not intended for this host
    error InvalidDestination();

    // Provided request has timed out
    error RequestTimedOut();

    // Provided request has not timed out
    error RequestNotTimedOut();

    // Provided bid cannot usurp the existing bid
    error BidTooHigh();

    // Unfortunately no one has bid to fulfil this request
    error NoExistingBid();

    // Provided request already fulfilled
    error RequestAlreadyFulfilled();

    // Unexpected zero address
    error ZeroAddress();

    // Provided amount was invalid
    error InvalidAmount();

    // Provided token was unknown
    error UnknownAsset();

    // Protocol invariant violated
    error InconsistentState();

    // Provided address didn't fit address type size
    error InvalidAddressLength();

    // restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (msg.sender != caller) revert UnauthorizedAction();
        _;
    }

    constructor(address admin) {
        _admin = admin;
    }

    // @dev initialize required parameters
    function init(TokenGatewayParamsExt memory teleportParams) public restrict(_admin) {
        _params = teleportParams.params;
        createAssets(teleportParams.assets);

        // infinite approval to save on gas
        SafeERC20.safeIncreaseAllowance(
            IERC20(IIsmpHost(_params.host).feeToken()),
            teleportParams.params.host,
            type(uint256).max
        );

        // admin can only call this once
        _admin = address(0);
    }

    // @dev Read the protocol parameters
    function params() external view returns (TokenGatewayParams memory) {
        return _params;
    }

    // @dev Fetch the address for an ERC20 asset
    function erc20(bytes32 assetId) external view returns (address) {
        return _erc20s[assetId];
    }

    // @dev Fetch the address for an ERC6160 asset
    function erc6160(bytes32 assetId) external view returns (address) {
        return _erc6160s[assetId];
    }

    // @dev Teleports a local ERC20/ERC6160 asset to the destination chain. Allows users to pay
    // the Hyperbridge fees in any ERC20 token that can be swapped for the swapped for the
    // `IIsmpHost.feeToken` using the local UniswapV2 router.
    //
    // @notice If a request times out, users can request a refund permissionlessly through
    // `HandlerV1.handlePostRequestTimeouts`.
    function teleport(TeleportParams memory teleportParams) public {
        if (teleportParams.to == bytes32(0)) revert ZeroAddress();
        if (teleportParams.amount == 0) revert InvalidAmount();

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
                    maxFee: teleportParams.maxFee,
                    assetId: teleportParams.assetId,
                    redeem: teleportParams.redeem,
                    data: teleportParams.data
                })
            )
            : abi.encode(
                Body({
                    from: fromBytes32,
                    to: teleportParams.to,
                    maxFee: teleportParams.maxFee,
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
            revert UnknownAsset();
        }

        uint256 fee = (IIsmpHost(_params.host).perByteFee() * data.length) + teleportParams.relayerFee;
        // only swap if the feeToken is not the token intended for fee
        if (feeToken != teleportParams.feeToken) {
            SafeERC20.safeTransferFrom(
                IERC20(teleportParams.feeToken),
                from,
                address(this),
                teleportParams.amountInMax
            );
            SafeERC20.safeIncreaseAllowance(
                IERC20(teleportParams.feeToken),
                _params.uniswapV2,
                teleportParams.amountInMax
            );

            address[] memory path = new address[](2);
            path[0] = teleportParams.feeToken;
            path[1] = feeToken;

            IUniswapV2Router(_params.uniswapV2).swapTokensForExactTokens(
                fee,
                teleportParams.amountInMax,
                path,
                address(this),
                block.timestamp
            );
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken), from, address(this), fee);
        }

        DispatchPost memory request = DispatchPost({
            dest: teleportParams.dest,
            to: abi.encodePacked(address(this)),
            body: data,
            timeout: teleportParams.timeout,
            fee: teleportParams.relayerFee,
            payer: msg.sender
        });
        bytes32 commitment = IIsmpHost(_params.host).dispatch(request);

        emit AssetTeleported({
            from: from,
            to: teleportParams.to,
            assetId: teleportParams.assetId,
            amount: teleportParams.amount,
            redeem: teleportParams.redeem,
            commitment: commitment
        });
    }

    // @dev Bid to fulfil an incoming asset. This will displace any pre-existing bid
    // if the liquidity fee is lower than said bid. This effectively creates a
    // race to the bottom for fees.
    //
    // @notice The request must not have expired, and must not have already been fulfilled.
    function bid(PostRequest calldata request, uint256 fee) public {
        // TokenGateway only accepts incoming assets from it's instances on other chains.
        if (!request.from.equals(abi.encodePacked(address(this)))) revert UnauthorizedAction();
        // Not sure why anyone would do this
        if (!request.dest.equals(IIsmpHost(_params.host).host())) revert UnauthorizedAction();
        // cannot bid on timed-out requests
        if (block.timestamp > request.timeout()) revert RequestTimedOut();

        bytes32 commitment = request.hash();
        // cannot bid on fulfilled requests
        if (IIsmpHost(_params.host).requestReceipts(commitment) != address(0)) revert RequestAlreadyFulfilled();

        Body memory body;
        if (request.body.length > BODY_BYTES_SIZE) {
            BodyWithCall memory bodyWithCall = abi.decode(request.body[1:], (BodyWithCall));
            body = Body({
                amount: bodyWithCall.amount,
                maxFee: bodyWithCall.maxFee,
                assetId: bodyWithCall.assetId,
                redeem: bodyWithCall.redeem,
                from: bodyWithCall.from,
                to: bodyWithCall.to
            });
        } else {
            body = abi.decode(request.body[1:], (Body));
        }

        if (body.redeem) revert UnauthorizedAction();

        address erc20Address = _erc20s[body.assetId];
        if (erc20Address == address(0)) revert UnknownAsset();

        LiquidityBid memory liquidityBid = _bids[commitment];

        // no existing bids
        if (liquidityBid.bidder == address(0)) {
            if (fee > body.maxFee) revert BidTooHigh();

            // transfer from bidder to this
            SafeERC20.safeTransferFrom(IERC20(erc20Address), msg.sender, address(this), body.amount - fee);
        } else {
            if (fee >= liquidityBid.fee) revert BidTooHigh();
            // refund previous bidder
            SafeERC20.safeTransfer(IERC20(erc20Address), liquidityBid.bidder, body.amount - liquidityBid.fee);

            // transfer from new bidder to this
            SafeERC20.safeTransferFrom(IERC20(erc20Address), msg.sender, address(this), body.amount - fee);
        }

        _bids[commitment] = LiquidityBid({bidder: msg.sender, fee: fee});

        // emit event
        emit BidPlaced({commitment: commitment, assetId: body.assetId, bid: fee, bidder: msg.sender});
    }

    // @dev This allows the bidder to refund their bids in the event that the request timed-out before
    // the bid could be fulfilled.
    function refundBid(PostRequest calldata request) public {
        // TokenGateway only accepts incoming assets from it's instances on other chains.
        if (!request.from.equals(abi.encodePacked(address(this)))) revert UnauthorizedAction();
        // Not sure why anyone would do this
        if (!request.dest.equals(IIsmpHost(_params.host).host())) revert UnauthorizedAction();
        // Cannot refund bids on requests which have not timed out, sorry.
        if (request.timeout() > block.timestamp) revert RequestNotTimedOut();

        bytes32 commitment = request.hash();
        // cannot refund bids for fulfilled requests
        if (IIsmpHost(_params.host).requestReceipts(commitment) != address(0)) revert RequestAlreadyFulfilled();

        LiquidityBid memory liquidityBid = _bids[commitment];
        if (liquidityBid.bidder == address(0)) revert NoExistingBid();

        Body memory body;
        if (request.body.length > BODY_BYTES_SIZE) {
            BodyWithCall memory bodyWithCall = abi.decode(request.body[1:], (BodyWithCall));
            body = Body({
                amount: bodyWithCall.amount,
                maxFee: bodyWithCall.maxFee,
                assetId: bodyWithCall.assetId,
                redeem: bodyWithCall.redeem,
                from: bodyWithCall.from,
                to: bodyWithCall.to
            });
        } else {
            body = abi.decode(request.body[1:], (Body));
        }

        address erc20Address = _erc20s[body.assetId];

        // can only happen if someone bids on an asset right before it was deregistered.
        // In this case, the asset will need to be re-registered
        if (erc20Address == address(0)) revert UnknownAsset();

        SafeERC20.safeTransfer(IERC20(erc20Address), liquidityBid.bidder, body.amount - liquidityBid.fee);

        delete _bids[commitment];

        emit BidRefunded({commitment: commitment, assetId: body.assetId, bidder: msg.sender});
    }

    function onAccept(IncomingPostRequest calldata incoming) external override restrict(_params.host) {
        OnAcceptActions action = OnAcceptActions(uint8(incoming.request.body[0]));

        if (action == OnAcceptActions.IncomingAsset) {
            if (incoming.request.body.length > BODY_BYTES_SIZE) {
                handleIncomingAssetWithCall(incoming);
            } else {
                handleIncomingAssetWithoutCall(incoming);
            }
        } else if (action == OnAcceptActions.GovernanceAction) {
            handleGovernance(incoming.request);
        } else if (action == OnAcceptActions.CreateAsset) {
            handleCreateAsset(incoming.request);
        } else if (action == OnAcceptActions.DeregisterAsset) {
            deregisterAssets(incoming.request);
        } else if (action == OnAcceptActions.ChangeAssetAdmin) {
            changeAssetAdmin(incoming.request);
        } else if (action == OnAcceptActions.NewContractInstance) {
            handleNewContractInstance(incoming.request);
        }
    }

    // @dev Triggered when a previously sent out request is confirmed to be timed-out by the IsmpHost.
    // @notice This means the funds could not be sent, we simply refund the user's assets here.
    function onPostRequestTimeout(PostRequest calldata request) external override restrict(_params.host) {
        Body memory body;
        if (request.body.length > BODY_BYTES_SIZE) {
            BodyWithCall memory bodyWithCall = abi.decode(request.body[1:], (BodyWithCall));
            body = Body({
                amount: bodyWithCall.amount,
                assetId: bodyWithCall.assetId,
                maxFee: bodyWithCall.maxFee,
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

        emit AssetRefunded({commitment: request.hash(), beneficiary: from, amount: body.amount, assetId: body.assetId});
    }

    function handleIncomingAssetWithoutCall(IncomingPostRequest calldata incoming) private {
        // TokenGateway only accepts incoming assets from it's instances on other chains.
        if (!incoming.request.from.equals(abi.encodePacked(address(this)))) {
            // Check if known address
            if (_instances[keccak256(incoming.request.source)] != bytesToAddress(incoming.request.from)) {
                revert UnauthorizedAction();
            }
        }

        Body memory body = abi.decode(incoming.request.body[1:], (Body));
        bytes32 commitment = incoming.request.hash();
        handleIncomingAsset(body, commitment);

        emit AssetReceived({
            commitment: commitment,
            beneficiary: bytes32ToAddress(body.to),
            from: body.from,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    function handleIncomingAssetWithCall(IncomingPostRequest calldata incoming) private {
        // TokenGateway only accepts incoming assets from it's instances on other chains.
        if (!incoming.request.from.equals(abi.encodePacked(address(this)))) {
            // Check if known address
            if (_instances[keccak256(incoming.request.source)] != bytesToAddress(incoming.request.from)) {
                revert UnauthorizedAction();
            }
        }

        BodyWithCall memory body = abi.decode(incoming.request.body[1:], (BodyWithCall));
        bytes32 commitment = incoming.request.hash();
        handleIncomingAsset(
            Body({
                amount: body.amount,
                maxFee: body.maxFee,
                assetId: body.assetId,
                redeem: body.redeem,
                from: body.from,
                to: body.to
            }),
            commitment
        );

        // dispatching low level call
        CallDispatcherParams memory dispatcherParams = abi.decode(body.data, (CallDispatcherParams));
        ICallDispatcher(_params.dispatcher).dispatch(dispatcherParams);

        emit AssetReceived({
            commitment: commitment,
            beneficiary: bytes32ToAddress(body.to),
            from: body.from,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    function handleIncomingAsset(Body memory body, bytes32 commitment) private {
        address _erc20 = _erc20s[body.assetId];
        address _erc6160 = _erc6160s[body.assetId];

        if (_erc20 != address(0) && body.redeem) {
            // a relayer/user is redeeming the native asset
            SafeERC20.safeTransfer(IERC20(_erc20), bytes32ToAddress(body.to), body.amount);
        } else if (_erc20 != address(0) && _erc6160 != address(0) && !body.redeem) {
            // user is swapping, fetch the bid
            LiquidityBid memory liquidityBid = _bids[commitment];
            if (liquidityBid.bidder == address(0)) revert NoExistingBid();

            uint256 value = body.amount - liquidityBid.fee;
            SafeERC20.safeTransfer(IERC20(_erc20), bytes32ToAddress(body.to), value);
            // hand the bidder the receipt so they can redeem the asset on the source chain
            IERC6160Ext20(_erc6160).mint(liquidityBid.bidder, body.amount);
            emit RequestFulfilled({bidder: liquidityBid.bidder, amount: value, assetId: body.assetId});
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(bytes32ToAddress(body.to), body.amount);
        } else {
            revert UnknownAsset();
        }
    }

    function handleGovernance(PostRequest calldata request) private {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        TokenGatewayParams memory params = abi.decode(request.body[1:], (TokenGatewayParams));

        emit ParamsUpdated({oldParams: _params, newParams: params});

        _params = params;
    }

    function handleCreateAsset(PostRequest calldata request) private {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        AssetMetadata[] memory assets = new AssetMetadata[](1);
        assets[0] = abi.decode(request.body[1:], (AssetMetadata));
        createAssets(assets);
    }

    // Deregisters the asset from TokenGateway. Users will be unable to bridge the asset
    // through TokenGateway once they are deregistered
    function deregisterAssets(PostRequest calldata request) private {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        DeregsiterAsset memory deregister = abi.decode(request.body[1:], (DeregsiterAsset));
        uint256 length = deregister.assetIds.length;
        for (uint256 i = 0; i < length; ++i) {
            delete _erc20s[deregister.assetIds[i]];
            delete _erc6160s[deregister.assetIds[i]];

            emit AssetRemoved({assetId: deregister.assetIds[i]});
        }
    }

    // Changes the asset admin from this contract to some other address. Changing the admin to a
    // zero address is disallowed for safety reasons
    function changeAssetAdmin(PostRequest calldata request) private {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        ChangeAssetAdmin memory asset = abi.decode(request.body[1:], (ChangeAssetAdmin));
        address erc6160Address = _erc6160s[asset.assetId];

        if (asset.newAdmin == address(0)) revert ZeroAddress();
        if (erc6160Address == address(0)) revert UnknownAsset();

        IERC6160Ext20(erc6160Address).changeAdmin(asset.newAdmin);

        emit AssetAdminChanged({asset: erc6160Address, newAdmin: asset.newAdmin});
    }

    function handleNewContractInstance(PostRequest calldata request) private {
        if (!request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        ContractInstance memory instance = abi.decode(request.body[1:], (ContractInstance));

        _instances[keccak256(instance.chain)] = instance.moduleId;

        emit NewContractInstance({chain: string(instance.chain), moduleId: instance.moduleId});
    }

    // Creates a new entry for the provided asset in the mappings. If there's no existing
    // ERC6160 address provided, then a contract for the asset is created.
    function createAssets(AssetMetadata[] memory assets) private {
        uint256 length = assets.length;
        for (uint256 i = 0; i < length; ++i) {
            AssetMetadata memory asset = assets[i];
            bytes32 identifier = keccak256(bytes(asset.symbol));
            string memory symbol = asset.symbol;
            if (asset.erc20 != address(0)) {
                symbol = string.concat(symbol, ".h");
            }
            if (asset.erc6160 == address(0)) {
                ERC6160Ext20 erc6160Asset = new ERC6160Ext20{salt: identifier}(address(this), asset.name, symbol);
                asset.erc6160 = address(erc6160Asset);
                if (asset.beneficiary != address(0) && asset.initialSupply != 0) {
                    erc6160Asset.mint(asset.beneficiary, asset.initialSupply);
                }
            }
            _erc20s[identifier] = asset.erc20;
            _erc6160s[identifier] = asset.erc6160;

            emit AssetRegistered({
                erc20: asset.erc20,
                erc6160: asset.erc6160,
                name: asset.name,
                symbol: asset.symbol,
                assetId: identifier,
                beneficiary: asset.beneficiary,
                initialSupply: asset.initialSupply
            });
        }
    }

    function addressToBytes32(address _address) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }

    function bytes32ToAddress(bytes32 _bytes) internal pure returns (address) {
        return address(uint160(uint256(_bytes)));
    }

    function bytesToAddress(bytes memory _bytes) internal pure returns (address addr) {
        if (_bytes.length != 20) {
            revert InvalidAddressLength();
        }
        assembly {
            addr := mload(add(_bytes, 20))
        }
    }
}
