// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {IDispatcher, DispatchPost} from "ismp/IDispatcher.sol";
import {IIsmpHost} from "ismp/IIsmpHost.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {BaseIsmpModule, PostRequest, IncomingPostRequest} from "ismp/IIsmpModule.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {IERC6160Ext20} from "ERC6160/interfaces/IERC6160Ext20.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
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
    // List of supported assets
    Asset[] assets;
}

struct Asset {
    /// ERC20 token contract address for the asset
    address erc20;
    /// ERC6160 token contract address for the asset
    address erc6160;
    /// Asset's identifier
    bytes32 identifier;
    // Associated fees for this asset
    AssetFees fees;
}

struct AssetFees {
    // Fee percentage paid to relayers for this asset
    uint256 relayerFeePercentage;
    // Fee percentage paid to the protocol for this asset
    uint256 protocolFeePercentage;
}

enum OnAcceptActions {
    // Incoming asset from a chain
    IncomingAsset,
    // Governance action to update protocol parameters
    GovernanceAction
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
}

/// The TokenGateway allows users send either ERC20 or ERC6160 tokens
/// using Hyperbridge as a message-passing layer.
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
    event LiquidityProvided(address relayer, uint256 amount, bytes32 assetId);
    // User has received some assets
    event AssetReceived(bytes source, uint256 nonce, address beneficiary, uint256 amount, bytes32 assetId);
    // User has sent some assets
    event AssetTeleported(
        address from, bytes32 to, uint256 amount, bytes32 assetId, bool redeem, bytes32 requestCommitment
    );
    // User assets could not be delivered and have been refunded.
    event AssetRefunded(address beneficiary, uint256 amount, bytes32 assetId, bytes dest, uint256 nonce);

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != _params.host) {
            revert("TokenGateway: Unauthorized action");
        }
        _;
    }

    // restricts call to `admin`
    modifier onlyAdmin() {
        if (msg.sender != _admin) {
            revert("TokenGateway: Unauthorized action");
        }
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

    function erc20(bytes32 assetId) external view returns (address) {
        return _erc20s[assetId];
    }

    function erc6160(bytes32 assetId) external view returns (address) {
        return _erc6160s[assetId];
    }

    function fees(bytes32 assetId) external view returns (AssetFees memory) {
        return _fees[assetId];
    }

    function teleport(TeleportParams memory teleportParams) public {
        require(teleportParams.to != bytes32(0), "Burn your funds some other way");
        require(teleportParams.amount > 100_000, "Amount too low");
        require(teleportParams.feeToken != address(0), "Fee token not selected");

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
            require(
                IERC20(_erc20).transferFrom(from, address(this), teleportParams.amount), "Insufficient user balance"
            );
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).burn(from, teleportParams.amount, "");
        } else {
            revert("Unknown Token Identifier");
        }

        uint256 fee = (IIsmpHost(_params.host).perByteFee() * data.length) + teleportParams.fee;
        // only swap if the feeToken is not the token intended for fee
        if (feeToken != teleportParams.feeToken) {
            address[] memory path = new address[](2);
            path[0] = teleportParams.feeToken;
            path[1] = feeToken;

            require(
                IERC20(teleportParams.feeToken).transferFrom(from, address(this), teleportParams.amountInMax),
                "insufficient funds for intended fee token"
            );
            require(
                IERC20(teleportParams.feeToken).approve(_params.uniswapV2, teleportParams.amountInMax), "approve failed"
            );
            IUniswapV2Router(_params.uniswapV2).swapTokensForExactTokens(
                fee, teleportParams.amountInMax, path, address(this), block.timestamp
            );
        } else {
            require(IERC20(feeToken).transferFrom(from, address(this), fee), "user has insufficient funds");
        }

        // approve the host with the exact amount
        require(IERC20(feeToken).approve(_params.host, fee), "approve failed");
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
        } else {
            revert("Unknown Action");
        }
    }

    function onPostRequestTimeout(PostRequest calldata request) external override onlyIsmpHost {
        // The funds could not be sent, this would allow users to get their funds back.
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
            require(IERC20(_erc20).transfer(from, body.amount), "Gateway: Insufficient Balance");
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(from, body.amount, "");
        } else {
            revert("Gateway: Inconsistent State");
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
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
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
        /// TokenGateway only accepts incoming assets from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
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
            require(IERC20(_erc20).transfer(to, transferredAmount), "Gateway: Insufficient Balance");
        } else if (_erc20 != address(0) && _erc6160 != address(0) && !redeem) {
            // user is swapping, relayers should double as liquidity providers.
            uint256 transferredAmount = amount - relayerLiquidityFee(assetId, amount);
            require(
                IERC20(_erc20).transferFrom(relayer, to, transferredAmount), "Gateway: Insufficient relayer balance"
            );
            // hand the relayer the erc6160, so they can redeem on the source chain
            IERC6160Ext20(_erc6160).mint(relayer, amount, "");

            emit LiquidityProvided({relayer: relayer, amount: transferredAmount, assetId: assetId});
        } else if (_erc6160 != address(0)) {
            IERC6160Ext20(_erc6160).mint(to, amount, "");
        } else {
            revert("Gateway: Unknown Token Identifier");
        }
    }

    function handleGovernance(PostRequest calldata request) private {
        // only hyperbridge can do this
        require(request.source.equals(IIsmpHost(_params.host).hyperbridge()), "Unauthorized request");
        TokenGatewayParamsExt memory teleportParams = abi.decode(request.body[1:], (TokenGatewayParamsExt));

        _params = teleportParams.params;
        setAssets(teleportParams.assets);
    }

    function relayerLiquidityFee(bytes32 assetId, uint256 amount) private view returns (uint256 liquidityFee) {
        liquidityFee = (amount * _fees[assetId].relayerFeePercentage) / 100_000;
    }

    function protocolFee(bytes32 assetId, uint256 amount) private view returns (uint256 redeemFee) {
        redeemFee = (amount * _fees[assetId].protocolFeePercentage) / 100_000;
    }

    function setAssets(Asset[] memory assets) private {
        uint256 length = assets.length;
        for (uint256 i = 0; i < length; i++) {
            Asset memory asset = assets[i];
            _erc20s[asset.identifier] = asset.erc20;
            _erc6160s[asset.identifier] = asset.erc6160;
            _fees[asset.identifier] = asset.fees;
        }
    }

    function addressToBytes32(address _address) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }

    function bytes32ToAddress(bytes32 _bytes) internal pure returns (address) {
        return address(uint160(uint256(_bytes)));
    }
}
