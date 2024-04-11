// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {IDispatcher, DispatchPost} from "ismp/IDispatcher.sol";
import {IIsmpHost} from "ismp/IIsmpHost.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {BaseIsmpModule, PostRequest} from "ismp/IIsmpModule.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {IERC6160Ext20} from "ERC6160/interfaces/IERC6160Ext20.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {CallDispatcher, ICallDispatcher} from "./CallDispatcher.sol";

import {IUniswapV2Router} from "../interfaces/IUniswapV2Router.sol";
import {IAllowanceTransfer} from "permit2/interfaces/IAllowanceTransfer.sol";

struct TeleportPermit {
    // permit details
    IAllowanceTransfer.PermitSingle permit;
    // permit authorization signature
    bytes signature;
}

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
    // Permit Details & Signature for host to spend feeToken
    TeleportPermit hostPermit;
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
    // StateMachine identifier for hyperbridge
    bytes hyperbridge;
    // address of the IsmpHost contract on this chain
    address host;
    // local uniswap router
    address uniswapV2;
    // dispatcher for delegating external calls
    address dispatcher;
    // Permit2 contract address
    address permit2;
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

    // User has received some assets, source chain & nonce
    event AssetReceived(bytes source, uint256 nonce);
    event Teleport(bytes32 from, bytes32 to, uint256 amount, bool redeem, bytes32 requestCommitment);

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
    function init(TokenGatewayParamsExt memory params) public onlyAdmin {
        _params = params.params;
        setAssets(params.assets);

        // admin can only call this once
        _admin = address(0);
    }

    function teleport(TeleportParams memory params) public {
        require(params.to != bytes32(0), "Burn your funds some other way");
        require(params.amount > 100_000, "Amount too low");
        require(params.feeToken != address(0), "Fee token not selected");

        address from = msg.sender;
        bytes32 fromBytes32 = addressToBytes32(msg.sender);
        address erc20 = _erc20s[params.assetId];
        address erc6160 = _erc6160s[params.assetId];
        address feeToken = IIsmpHost(_params.host).feeToken();

        bytes memory data = params.data.length > 0
            ? abi.encode(
                BodyWithCall({
                    from: fromBytes32,
                    to: params.to,
                    amount: params.amount,
                    assetId: params.assetId,
                    redeem: params.redeem,
                    data: params.data
                })
            )
            : abi.encode(
                Body({
                    from: fromBytes32,
                    to: params.to,
                    amount: params.amount,
                    assetId: params.assetId,
                    redeem: params.redeem
                })
            );

        if (erc20 != address(0) && !params.redeem) {
            require(IERC20(erc20).transferFrom(from, address(this), params.amount), "Insufficient user balance");

            // only swap if the feeToken is not the token intended for fee
            if (feeToken != params.feeToken) {
                // Calculate output fee in the fee token before swap:
                uint256 fee = (IIsmpHost(_params.host).perByteFee() * data.length + 1) + params.fee;
                require(handleSwap(from, params.feeToken, feeToken, fee, params.amountInMax), "Token swap failed");
            }
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).burn(from, params.amount, "");
        } else {
            revert("Unknown Token Identifier");
        }

        // permit the host with the exact amount
        IAllowanceTransfer(_params.permit2).permit(from, params.hostPermit.permit, params.hostPermit.signature);
        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: abi.encodePacked(address(this)),
            body: bytes.concat(hex"00", data), // add enum variant for body
            timeout: params.timeout,
            fee: params.fee,
            payer: msg.sender
        });
        bytes32 commitment = IDispatcher(_params.host).dispatch(request);

        emit Teleport({
            from: fromBytes32,
            to: params.to,
            amount: params.amount,
            redeem: params.redeem,
            requestCommitment: commitment
        });
    }

    function onAccept(PostRequest calldata request) external override onlyIsmpHost {
        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));

        if (action == OnAcceptActions.IncomingAsset) {
            if (request.body.length > BODY_BYTES_SIZE) {
                handleIncomingAssetWithCall(request);
            } else {
                handleIncomingAssetWithoutCall(request);
            }
        } else if (action == OnAcceptActions.GovernanceAction) {
            handleGovernance(request);
        } else {
            revert("Unknown Action");
        }
    }

    function onPostRequestTimeout(PostRequest calldata request) external override onlyIsmpHost {
        // The funds could not be sent, this would allow users to get their funds back.
        // todo: test this with BodyWithCall
        Body memory body = abi.decode(request.body[1:161], (Body));

        address erc20 = _erc20s[body.assetId];
        address erc6160 = _erc6160s[body.assetId];
        address fromAddress = bytes32ToAddress(body.from);

        if (erc20 != address(0) && !body.redeem) {
            require(IERC20(erc20).transfer(fromAddress, body.amount), "Gateway: Insufficient Balance");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).mint(fromAddress, body.amount, "");
        } else {
            revert("Gateway: Inconsistent State");
        }
    }

    function handleIncomingAssetWithoutCall(PostRequest calldata request) private {
        /// TokenGateway only accepts incoming assets from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
        Body memory body = abi.decode(request.body[1:], (Body));
        address toAddress = bytes32ToAddress(body.to);
        _handleIncomingAsset(body.assetId, body.redeem, body.amount, toAddress);

        emit AssetReceived(request.source, request.nonce);
    }

    function handleIncomingAssetWithCall(PostRequest calldata request) private {
        /// TokenGateway only accepts incoming assets from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
        BodyWithCall memory body = abi.decode(request.body[1:], (BodyWithCall));
        address toAddress = bytes32ToAddress(body.to);
        _handleIncomingAsset(body.assetId, body.redeem, body.amount, toAddress);
        // dispatching low level call
        ICallDispatcher(_params.dispatcher).dispatch(toAddress, body.data);

        emit AssetReceived(request.source, request.nonce);
    }

    function _handleIncomingAsset(bytes32 assetId, bool redeem, uint256 amount, address to) private {
        address erc20 = _erc20s[assetId];
        address erc6160 = _erc6160s[assetId];

        if (erc20 != address(0) && redeem) {
            // a relayer/user is redeeming the native asset
            uint256 transferredAmount = amount - protocolFee(assetId, amount);
            require(IERC20(erc20).transfer(to, transferredAmount), "Gateway: Insufficient Balance");
        } else if (erc20 != address(0) && erc6160 != address(0) && !redeem) {
            // user is swapping, relayers should double as liquidity providers.
            uint256 transferredAmount = amount - relayerLiquidityFee(assetId, amount);
            require(
                // we assume that the relayer is an EOA
                IERC20(erc20).transferFrom(tx.origin, to, transferredAmount),
                "Gateway: Insufficient relayer balance"
            );

            emit LiquidityProvided({relayer: tx.origin, amount: transferredAmount, assetId: assetId});

            // hand the relayer the erc6160, so they can redeem on the source chain
            IERC6160Ext20(erc6160).mint(tx.origin, amount, "");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).mint(to, amount, "");
        } else {
            revert("Gateway: Unknown Token Identifier");
        }
    }

    function handleGovernance(PostRequest calldata request) private {
        // only hyperbridge can do this
        require(request.source.equals(_params.hyperbridge), "Unauthorized request");
        TokenGatewayParamsExt memory params = abi.decode(request.body[2:], (TokenGatewayParamsExt));

        _params = params.params;
        setAssets(params.assets);
    }

    function handleSwap(
        address sender,
        address fromToken,
        address toToken,
        uint256 toTokenAmountOut,
        uint256 amountInMax
    ) private returns (bool) {
        address[] memory path = new address[](2);
        path[0] = fromToken;
        path[1] = toToken;

        require(IERC20(fromToken).transferFrom(sender, address(this), amountInMax), "insufficient intended fee token");
        require(IERC20(fromToken).approve(address(_params.uniswapV2), amountInMax), "approve failed.");
        IUniswapV2Router(_params.uniswapV2).swapTokensForExactTokens(
            toTokenAmountOut, amountInMax, path, sender, block.timestamp + 300
        );

        return true;
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
