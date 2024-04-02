// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {IDispatcher, DispatchPost} from "ismp/IDispatcher.sol";
import {IIsmpHost} from "ismp/IIsmpHost.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {BaseIsmpModule, PostRequest} from "ismp/IIsmpModule.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";
import {IERC6160Ext20} from "ERC6160/interfaces/IERC6160Ext20.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";

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
    address to;
    // The Erc20 token to be used to swap for a fee
    address feeToken;
    // recipient state machine
    bytes dest;
    // timeout in seconds
    uint64 timeout;
}

struct Body {
    // amount to be sent
    uint256 amount;
    // The token identifier
    bytes32 assetId;
    // flag to redeem the erc20 asset on the destination
    bool redeem;
    // sender address
    address from;
    // recipient address
    address to;
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
/// Incoming asset from a chain
{
    IncomingAsset,
    /// Governance actions
    GovernanceAction
}

enum GovernanceActions
{
    /// Some new assets are now supported by gateway
    NewAssets,
    /// Governance has decided to adjust liquidity fee paid to relayers
    AdjustLiquidityFee,
    ///  Governance has decided to adjust it's own protocol fee
    AdjustProtocolFee,
    /// Assets can be removed from the gateway through governance action
    RemoveAsset
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

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;
    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;

    // User has received some assets, source chain & nonce
    event AssetReceived(bytes source, uint256 nonce);
    event Teleport(address from, bytes dest, uint256 amount, uint256 fee, uint64 timeout);

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

        setAssets(initialParams.assets);

        // admin can only call this once
        _admin = address(0);
    }

    function teleport(TeleportParams memory params) public {
        require(params.to != address(0), "Burn your funds some other way");
        require(params.amount > 100_000, "Amount too low");
        require(params.feeToken != address(0), "Fee token not selected");

        address from = msg.sender;
        address erc20 = _erc20s[params.assetId];
        address erc6160 = _erc6160s[params.assetId];
        address feeToken = IIsmpHost(_host).dai();

        if (erc20 != address(0) && !params.redeem) {
            require(IERC20(erc20).transferFrom(from, address(this), params.amount), "Insufficient user balance");

            // Calculate output fee in DAI before swap:
            // We can use swapTokensForExactTokens() on Uniswap since we know the output amount
            uint256 _fee = calculateBridgeFee(params.fee);

            // only swap if the feeToken is not the token intended for fee
            if (feeToken != params.feeToken) {
                require(handleSwap(from, params.feeToken, feeToken, _fee), "Token swap failed");
            }
        } else if (erc6160 != address(0)) {
            // we're sending an erc6160 asset so we should redeem on the destination if we can.
            IERC6160Ext20(erc6160).burn(from, params.amount, "");
        } else {
            revert("Unknown Token Identifier");
        }

        bytes memory data = abi.encode(
            Body({from: from, to: params.to, amount: params.amount, assetId: params.assetId, redeem: params.redeem})
        );

        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: abi.encodePacked(address(this)),
            // add enum variant for body
            body: bytes.concat(hex"00", data),
            timeout: params.timeout,
            fee: params.fee,
            gaslimit: uint64(0),
            payer: msg.sender
        });

        // Your money is now on its way
        IDispatcher(_host).dispatch(request);

        emit Teleport(from, params.dest, params.amount, params.fee, params.timeout);
    }

    function onAccept(PostRequest calldata request) external override onlyIsmpHost {
        OnAcceptActions action = OnAcceptActions(uint8(request.body[0]));

        if (action == OnAcceptActions.IncomingAsset) {
            handleIncomingAsset(request);
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

        if (erc20 != address(0) && !body.redeem) {
            require(IERC20(erc20).transfer(body.from, body.amount), "Gateway: Insufficient Balance");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).mint(body.from, body.amount, "");
        } else {
            revert("Gateway: Inconsistent State");
        }
    }

    function handleIncomingAsset(PostRequest calldata request) private {
        /// TokenGateway only accepts incoming assets from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
        Body memory body = abi.decode(request.body[1:], (Body));

        address erc20 = _erc20s[body.assetId];
        address erc6160 = _erc6160s[body.assetId];

        if (erc20 != address(0) && body.redeem) {
            // a relayer/user is redeeming the native asset
            uint256 _protocolRedeemFee = calculateProtocolFee(body.amount);
            uint256 _amountToTransfer = body.amount - _protocolRedeemFee;

            require(IERC20(erc20).transfer(body.to, _amountToTransfer), "Gateway: Insufficient Balance");
        } else if (erc20 != address(0) && erc6160 != address(0) && !body.redeem) {
            // user is swapping, relayers should double as liquidity providers.
            uint256 _protocolLiquidityFee = calculateRelayerLiquidityFee(body.amount);
            uint256 _amountToTransfer = body.amount - _protocolLiquidityFee;

            require(
                // we assume that the relayer is an EOA
                IERC20(erc20).transferFrom(tx.origin, body.to, _amountToTransfer),
                "Gateway: Insufficient relayer balance"
            );
            
            emit LiquidityProvided(tx.origin, _amountToTransfer);

            // hand the relayer the erc6160, so they can redeem on the source chain
            IERC6160Ext20(erc6160).mint(tx.origin, body.amount, "");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).mint(body.to, body.amount, "");
        } else {
            revert("Gateway: Unknown Token Identifier");
        }

        emit AssetReceived(request.source, request.nonce);
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

    function handleSwap(address _sender, address _fromToken, address _toToken, uint256 _toTokenAmountOut)
        private
        returns (bool)
    {
        address[] memory path = new address[](2);
        path[0] = _fromToken;
        path[1] = _toToken;

        uint256 _fromTokenAmountIn = _uniswapV2Router.getAmountsIn(_toTokenAmountOut, path)[0];

        // How do we handle cases of slippage - Todo: Handle Slippage
        require(
            IERC20(_fromToken).transferFrom(_sender, address(this), _fromTokenAmountIn),
            "insufficient intended fee token"
        );
        require(IERC20(_fromToken).approve(address(_uniswapV2Router), _fromTokenAmountIn), "approve failed.");

        _uniswapV2Router.swapTokensForExactTokens(_toTokenAmountOut, _fromTokenAmountIn, path, _sender, block.timestamp);

        return true;
    }

    function calculateBridgeFee(uint256 _relayerFee) private view returns (uint256) {
        // Multiply the perByteFee by the byte size of the body struct, and sum with relayer fee
        uint256 _fee = (IIsmpHost(_host).perByteFee() * BODY_BYTES_SIZE) + _relayerFee;

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
}
