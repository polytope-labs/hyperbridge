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

struct InitParams {
    /// IsmpHost address
    address host;
    /// Address for the Uniswapv2 router contract
    address uniswapV2Router;
    /// Hyperbridge state machine identifier
    bytes hyperbridge;
    /// Fee percentage paid to relayers
    uint256 relayerFeePercentage;
    /// Fee percentage paid to the protocol
    uint256 protocolFeePercentage;
    /// List of supported assets
    Asset[] assets;
    /// Address of the call dispatcher contract
    address callDispatcher;
}

struct Asset {
    /// ERC20 token contract address for the asset
    address erc20;
    /// ERC6160 token contract address for the asset
    address erc6160;
    /// Asset's identifier
    bytes32 identifier;
}

enum OnAcceptActions
// Incoming asset from a chain
{
    IncomingAsset,
    // Governance actions
    GovernanceAction
}

enum GovernanceActions
// Some new assets are now supported by gateway
{
    NewAssets,
    // Governance has decided to adjust liquidity fee paid to relayers
    AdjustLiquidityFee,
    //  Governance has decided to adjust it's own protocol fee
    AdjustProtocolFee
}

// Abi-encoded size of Body struct
uint256 constant BODY_BYTES_SIZE = 161;

/// The TokenGateway allows users send either ERC20 or ERC6160 tokens
/// using Hyperbridge as a message-passing layer.
contract TokenGateway is BaseIsmpModule {
    event LiquidityProvided(address relayer, uint256 amount);

    using Bytes for bytes;

    /// StateMachine identifier for hyperbridge
    bytes private _hyperbridge;
    /// address of the IsmpHost contract on this chain
    address private _host;
    /// admin account
    address private _admin;
    /// Fee percentage paid to relayers
    uint256 private _relayerFeePercentage;
    /// Fee percentage paid to the protocol
    uint256 private _protocolFeePercentage;
    /// local uniswap router
    IUniswapV2Router private _uniswapV2Router;
    /// call dispatcher
    ICallDispatcher private _dispatcher;

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;
    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;

    // todo: map assetId to liquidity fee, so fees are configurable on a per asset basis

    // User has received some assets
    event AssetReceived(bytes source, uint256 nonce, address beneficiary, uint256 amount, bytes32 assetId);
    // User has sent some assets
    event AssetTeleported(address from, bytes32 to, uint256 amount, bool redeem, bytes32 requestCommitment);
    // User assets could not be delivered and has been refunded.
    event AssetRefunded(address beneficiary, uint256 amount, bytes32 assetId, bytes dest, uint256 nonce);

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != _host) {
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
    function init(InitParams memory initialParams) public onlyAdmin {
        _host = initialParams.host;
        _hyperbridge = initialParams.hyperbridge;
        _protocolFeePercentage = initialParams.protocolFeePercentage;
        _relayerFeePercentage = initialParams.relayerFeePercentage;
        _uniswapV2Router = IUniswapV2Router(initialParams.uniswapV2Router);
        _dispatcher = ICallDispatcher(initialParams.callDispatcher);

        setAssets(initialParams.assets);

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
        address feeToken = IIsmpHost(_host).feeToken();

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
                Body({from: fromBytes32, to: params.to, amount: params.amount, assetId: params.assetId, redeem: params.redeem})
            );

        if (erc20 != address(0) && !params.redeem) {
            require(IERC20(erc20).transferFrom(from, address(this), params.amount), "Insufficient user balance");

            // Calculate output fee in the fee token before swap:
            uint256 _fee = calculateBridgeFee(params.fee, data);

            // only swap if the feeToken is not the token intended for fee
            if (feeToken != params.feeToken) {
                require(handleSwap(from, params.feeToken, feeToken, _fee, params.amountInMax), "Token swap failed");
            }
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).burn(from, params.amount, "");
        } else {
            revert("Unknown Token Identifier");
        }

        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: abi.encodePacked(address(this)),
            body: bytes.concat(hex"00", data), // add enum variant for body
            timeout: params.timeout,
            fee: params.fee,
            payer: msg.sender
        });

        bytes32 commitment = IDispatcher(_host).dispatch(request);

        emit AssetTeleported({
            from: from,
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
        // The money could not be sent, this would allow users to get their money back.
        Body memory body = abi.decode(request.body[1:], (Body));

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

        emit AssetRefunded({
            beneficiary: fromAddress,
            amount: body.amount,
            assetId: body.assetId,
            dest: request.dest,
            nonce: request.nonce
        });
    }

    function handleIncomingAssetWithoutCall(PostRequest calldata request) private {
        /// TokenGateway only accepts incoming assets from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
        Body memory body = abi.decode(request.body[1:], (Body));

        address toAddress = bytes32ToAddress(body.to);

        _handleIncomingAsset(body.assetId, body.redeem, body.amount, toAddress);

        emit AssetReceived({
            source: request.source,
            nonce: request.nonce,
            beneficiary: toAddress,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    function handleIncomingAssetWithCall(PostRequest calldata request) private {
        /// TokenGateway only accepts incoming assets from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
        BodyWithCall memory body = abi.decode(request.body[1:], (BodyWithCall));

        address toAddress = bytes32ToAddress(body.to);

        _handleIncomingAsset(body.assetId, body.redeem, body.amount, toAddress);

        // dispatching low level call
        _dispatcher.dispatch(toAddress, body.data);

        emit AssetReceived({
            source: request.source,
            nonce: request.nonce,
            beneficiary: toAddress,
            amount: body.amount,
            assetId: body.assetId
        });
    }

    function _handleIncomingAsset(bytes32 assetId, bool redeem, uint256 amount, address to) private {
        address erc20 = _erc20s[assetId];
        address erc6160 = _erc6160s[assetId];

        if (erc20 != address(0) && redeem) {
            // a relayer/user is redeeming the native asset
            uint256 _protocolRedeemFee = calculateProtocolFee(amount);
            uint256 _amountToTransfer = amount - _protocolRedeemFee;

            require(IERC20(erc20).transfer(to, _amountToTransfer), "Gateway: Insufficient Balance");
        } else if (erc20 != address(0) && erc6160 != address(0) && !redeem) {
            // user is swapping, relayers should double as liquidity providers.
            uint256 _protocolLiquidityFee = calculateRelayerLiquidityFee(amount);
            uint256 _amountToTransfer = amount - _protocolLiquidityFee;

            require(
                // we assume that the relayer is an EOA
                IERC20(erc20).transferFrom(tx.origin, to, _amountToTransfer),
                "Gateway: Insufficient relayer balance"
            );

            emit LiquidityProvided(tx.origin, _amountToTransfer);

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
        require(request.source.equals(_hyperbridge), "Unauthorized request");
        GovernanceActions action = GovernanceActions(uint8(request.body[1]));

        if (action == GovernanceActions.NewAssets) {
            setAssets(abi.decode(request.body[2:], (Asset[])));
        } else if (action == GovernanceActions.AdjustLiquidityFee) {
            _relayerFeePercentage = abi.decode(request.body[2:], (uint256));
        } else if (action == GovernanceActions.AdjustProtocolFee) {
            _protocolFeePercentage = abi.decode(request.body[2:], (uint256));
        } else {
            revert("Unknown Action");
        }
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
        require(IERC20(fromToken).approve(address(_uniswapV2Router), amountInMax), "approve failed.");

        _uniswapV2Router.swapTokensForExactTokens(toTokenAmountOut, amountInMax, path, sender, block.timestamp + 300);

        return true;
    }

    function calculateBridgeFee(uint256 _relayerFee, bytes memory data) private view returns (uint256) {
        // Multiply the perByteFee by the byte size of the body struct, and sum with relayer fee
        uint256 _fee = (IIsmpHost(_host).perByteFee() * data.length + 1) + _relayerFee;

        return _fee;
    }

    function calculateRelayerLiquidityFee(uint256 _amount) private view returns (uint256 bridgeFee) {
        bridgeFee = (_amount * _relayerFeePercentage) / 100_000;
    }

    function calculateProtocolFee(uint256 _amount) private view returns (uint256 redeemFee) {
        redeemFee = (_amount * _protocolFeePercentage) / 100_000;
    }

    function setAssets(Asset[] memory assets) private {
        uint256 length = assets.length;
        for (uint256 i = 0; i < length; i++) {
            Asset memory asset = assets[i];

            _erc20s[asset.identifier] = asset.erc20;
            _erc6160s[asset.identifier] = asset.erc6160;
        }
    }

    function addressToBytes32(address _address) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(_address)));
    }

    function bytes32ToAddress(bytes32 _bytes) internal pure returns (address) {
        return address(uint160(uint256(_bytes)));
    }
}
