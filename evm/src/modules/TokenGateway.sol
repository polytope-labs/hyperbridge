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
    bytes32 tokenId;
    // recipient address
    address to;
    // recipient state machine
    bytes dest;
    // timeout in seconds
    uint64 timeout;
    // Redeem Erc20 on the destination?
    bool redeem;
    // The Erc20 token to be used to swap for a fee
    address tokenIntendedForFee;
}

struct Body {
    // amount to be sent
    uint256 amount;
    // The token identifier
    bytes32 tokenId;
    // flag to redeem the erc20 asset on the destination
    bool redeem;
    // sender address
    address from;
    // recipient address
    address to;
}

/// The TokenGateway allows users send either ERC20 or ERC6160 tokens
/// using Hyperbridge as a message-passing layer.
contract TokenGateway is BaseIsmpModule {
    using Bytes for bytes;

    /// ParaId of hyperbridge
    uint256 private _paraId;
    /// address of the IsmpHost contract on this chain
    address private host;
    /// admin account
    address private admin;
    /// local uniswap router
    IUniswapV2Router private uniswapV2Router;

    // Bytes size of Body struct
    uint256 private constant BODY_BYTES_SIZE = 160;

    // mapping of token identifier to erc6160 contracts
    mapping(bytes32 => address) private _erc6160s;
    // mapping of token identifier to erc20 contracts
    mapping(bytes32 => address) private _erc20s;
    // foreign to local asset identifier mapping
    mapping(bytes32 => bytes32) private _assets;

    // User has received some assets, source chain & nonce
    event AssetReceived(bytes source, uint256 nonce);

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != host) {
            revert("Unauthorized call");
        }
        _;
    }

    // restricts call to `admin`
    modifier onlyAdmin() {
        if (msg.sender != admin) {
            revert("Unauthorized call");
        }
        _;
    }

    enum OnAcceptActions {
        /// Incoming asset from a chain
        IncomingAsset,
        /// Governance actions
        GovernanceAction
    }

    enum GovernanceActions {
        /// A new asset is now supported by gateway
        NewAsset,
        /// Governance has decided to adjust liquidity fee paid to relayers
        AdjustLiquidityFee,
        ///  Governance has decided to adjust it's protocol fee
        AdjustProtocolFee
    }

    constructor(address _admin) {
        admin = _admin;
    }

    // initialize required parameters
    function initParams(address _host, address _uniswapV2Router) public onlyAdmin {
        host = _host;
        uniswapV2Router = IUniswapV2Router(_uniswapV2Router);
    }

    function teleport(TeleportParams memory params) public {
        require(host != address(0), "Gateway: Host is not set");
        require(address(uniswapV2Router) != address(0), "Gateway: Uniswap router not set");

        address from = msg.sender;
        address erc20 = _erc20s[params.tokenId];
        address erc6160 = _erc6160s[params.tokenId];
        address intendedTokenForFee = params.tokenIntendedForFee;
        address feeToken = IIsmpHost(host).dai();

        require(params.to != address(0), "Burn your funds some other way");
        require(params.amount > 0, "Gateway: Can't bridge zero value");
        require(intendedTokenForFee != address(0), "Intended fee token not selected");

        uint256 toBridge = params.amount;

        if (erc20 != address(0) && !params.redeem) {
            require(
                IERC20(erc20).transferFrom(from, address(this), params.amount), "Gateway: Insufficient user balance"
            );

            // Calculate output fee in DAI before swap: We can use swapTokensForExactTokens() on Uniswap since we know the output amount
            uint256 _fee = calculateProtocolBridgeFee(params.fee);

            // only swap if the feeToken is not the token intended for fee and if fee > 0
            if (feeToken != intendedTokenForFee && _fee > 0) {
                require(handleSwap(from, intendedTokenForFee, feeToken, _fee), "Token swap failed");
            }
        } else if (erc6160 != address(0)) {
            // we're sending an erc6160 asset so we should redeem on the destination if we can.
            IERC6160Ext20(erc6160).burn(from, params.amount, "");
        } else {
            revert("Gateway: Unknown Token Identifier");
        }

        bytes memory data = abi.encode(
            Body({from: from, to: params.to, amount: toBridge, tokenId: params.tokenId, redeem: params.redeem})
        );

        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: abi.encodePacked(address(this)),
            // add enum variant for body
            body: bytes.concat(hex"00", data),
            timeout: params.timeout,
            fee: params.fee,
            gaslimit: uint64(0)
        });

        // Your money is now on its way
        IDispatcher(host).dispatch(request);
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

    function onPostRequestTimeout(PostRequest memory request) external override onlyIsmpHost {
        // The money could not be sent, this would allow users to get their money back.
        Body memory body = abi.decode(request.body, (Body));

        address erc20 = _erc20s[body.tokenId];
        address erc6160 = _erc6160s[body.tokenId];

        if (erc20 != address(0) && !body.redeem) {
            require(IERC20(erc20).transfer(body.from, body.amount), "Gateway: Insufficient Balance");
        } else if (erc6160 != address(0)) {
            IERC6160Ext20(erc6160).mint(body.from, body.amount, "");
        } else {
            revert("Gateway: Inconsistent State");
        }
    }

    function handleIncomingAsset(PostRequest calldata request) private {
        /// TokenGateway only accepts asset requests from it's instances on other chains.
        require(request.from.equals(abi.encodePacked(address(this))), "Unauthorized request");
        Body memory body = abi.decode(request.body[1:], (Body));

        bytes32 localAsset = _assets[body.tokenId];
        address erc20 = _erc20s[localAsset];
        address erc6160 = _erc6160s[localAsset];

        // prefer to give the user erc20
        if (erc20 != address(0) && body.redeem) {
            // a relayer/user is redeeming the native asset
            // Perform 0.1% calculation and deduction here
            uint256 _protocolRedeemFee = calculateProtocolRedeemFee(body.amount);
            uint256 _amountToTransfer = body.amount - _protocolRedeemFee;

            require(IERC20(erc20).transfer(body.to, _amountToTransfer), "Gateway: Insufficient Balance");
        } else if (erc20 != address(0) && erc6160 != address(0) && !body.redeem) {
            // relayers double as liquidity providers.
            // Perform 0.3% calculation and deduction here
            uint256 _protocolLiquidityFee = calculateProtocolLiquidityFee(body.amount);
            uint256 _amountToTransfer = body.amount - _protocolLiquidityFee;

            require(
                IERC20(erc20).transferFrom(tx.origin, body.to, _amountToTransfer),
                "Gateway: Insufficient relayer balance"
            );

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
        require(request.source.equals(StateMachine.kusama(_paraId)), "Unauthorized request");
        GovernanceActions action = GovernanceActions(uint8(request.body[1]));

        if (action == GovernanceActions.NewAsset) {
            // do stuff
        } else if (action == GovernanceActions.AdjustLiquidityFee) {
            // do stuff
        } else if (action == GovernanceActions.AdjustProtocolFee) {
            // do more stuff
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

        uint256 _fromTokenAmountIn = uniswapV2Router.getAmountsIn(_toTokenAmountOut, path)[0];

        // How do we handle cases of slippage - Todo: Handle Slippage
        require(
            IERC20(_fromToken).transferFrom(_sender, address(this), _fromTokenAmountIn),
            "insufficient intended fee token"
        );
        require(IERC20(_fromToken).approve(address(uniswapV2Router), _fromTokenAmountIn), "approve failed.");

        uniswapV2Router.swapTokensForExactTokens(
            _toTokenAmountOut, _fromTokenAmountIn, path, tx.origin, block.timestamp
        );

        return true;
    }

    function calculateProtocolBridgeFee(uint256 _relayerFee) private view returns (uint256) {
        // Multiply the perByteFee by the byte size of the body struct, and sum with relayer fee
        uint256 _fee = (IIsmpHost(host).perByteFee() * BODY_BYTES_SIZE) + _relayerFee;

        return _fee;
    }

    function calculateProtocolLiquidityFee(uint256 _amount) private pure returns (uint256 bridgeFee_) {
        bridgeFee_ = (_amount * 300) / 100_000;
    }

    function calculateProtocolRedeemFee(uint256 _amount) private pure returns (uint256 redeemFee_) {
        redeemFee_ = (_amount * 100) / 100_000;
    }

    function setTokenIdentifiersERC20(bytes32 _tokenId, address _erc20) external onlyAdmin {
        _erc20s[_tokenId] = _erc20;
    }

    function setTokenIdentifiersERC6160(bytes32 _tokenId, address _erc6160) external onlyAdmin {
        _erc6160s[_tokenId] = _erc6160;
    }

    function setForeignTokenIdToLocalTokenId(bytes32 _foreignTokenId, bytes32 _localTokenId) external onlyAdmin {
        _assets[_foreignTokenId] = _localTokenId;
    }
}
