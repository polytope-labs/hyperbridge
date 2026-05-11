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

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";

import {IWrappedHyperFungibleToken} from "../interfaces/IHyperFungibleToken.sol";
import {PostRequest} from "../libraries/Message.sol";
import {DispatchPost, IDispatcher} from "../interfaces/IDispatcher.sol";
import {IncomingPostRequest} from "../interfaces/IApp.sol";
import {ICallDispatcher} from "../interfaces/ICallDispatcher.sol";
import {IWETH} from "../interfaces/IWETH.sol";
import {HyperApp} from "./HyperApp.sol";
import {HyperFungibleToken} from "./HyperFungibleToken.sol";


/**
 * @title WrappedHyperFungibleToken
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Cross-chain wrapper for existing ERC20 tokens.
 * Locks the underlying token on the source chain and mints/unlocks on the destination chain.
 *
 * @dev Inherits HyperApp for cross-chain message handling and Ownable for configuration.
 * The owner configures which chains this wrapper can communicate with, the address of the
 * corresponding deployment on each chain, and the underlying ERC20 token.
 *
 * Also supports native token wrapping: if `msg.value >= params.amount` during send, the
 * contract wraps the native token by treating the underlying ERC20 as WETH. This reverts
 * naturally if the underlying is not a WETH contract.
 *
 * Supports optional calldata execution on the destination chain via CallDispatcher,
 * enabling composable cross-chain interactions (e.g., transfer-and-swap).
 */
contract WrappedHyperFungibleToken is ERC165, HyperApp, Ownable, Pausable {
    /**
     * @title WrappedConfigOptions
     * @notice Configuration parameters for WrappedHyperFungibleToken
     */
    struct WrappedConfigOptions {
        /// @notice Address of the ISMP host contract on this chain
        address host;
        /// @notice Address of the CallDispatcher contract for executing calldata on receive
        address dispatcher;
        /// @notice Address of the underlying ERC20 token to wrap
        address underlying;
        /// @notice Whether the underlying token is WETH (enables native ETH refunds on timeout)
        bool isWeth;
    }

    using SafeERC20 for IERC20;

    /// @notice Thrown when the provided bytes are too short to extract an address
    error InvalidAddress(uint256 length);


    /// @notice Thrown when a native ETH transfer fails during timeout refund
    error TransferFailed();

    /// @notice Thrown when attempting to send to or receive from an unconfigured chain
    error UnsupportedChain();

    /**
     * @notice Thrown when the source address of an incoming message does not match the
     * expected contract address for that chain
     */
    error UnauthorizedSource();

    /// @notice Address of the ISMP host contract on this chain
    address internal _host;

    /// @notice Address of the CallDispatcher contract for executing destination calldata
    address internal _dispatcher;

    /// @notice The underlying ERC20 token being wrapped for cross-chain transfers
    address internal _underlying;

    /// @notice Whether the underlying token is WETH
    bool internal _isWeth;

    /**
     * @notice Maps chain identifiers to the module ID of the peer on that chain.
     * An empty value means the chain is not supported.
     */
    mapping(bytes => bytes) internal _supportedChains;

    /**
     * @notice Emitted when tokens are locked and a cross-chain transfer is dispatched
     * @param from The sender on the source chain
     * @param to The recipient on the destination chain
     * @param dest The destination chain identifier
     * @param amount The amount of tokens sent
     * @param commitment The ISMP request commitment hash for tracking
     */
    event Sent(address from, bytes to, string dest, uint256 amount, bytes32 commitment);

    /**
     * @notice Emitted when tokens are unlocked from an incoming cross-chain transfer
     * @param from The original sender on the source chain
     * @param to The recipient on this chain
     * @param source The source chain identifier
     * @param amount The amount of tokens unlocked
     */
    event Received(bytes from, address to, string source, uint256 amount);

    /**
     * @notice Emitted when tokens are refunded after a cross-chain transfer timeout
     * @param to The original sender being refunded
     * @param amount The amount of tokens refunded
     */
    event Refunded(address to, uint256 amount);

    /**
     * @notice Initializes the contract with the given owner
     * @param initialOwner The address that will own this contract
     */
    constructor(address initialOwner) Ownable(initialOwner) {}

    /**
     * @notice Returns the ISMP host address for this chain
     * @return The host contract address
     */
    function host() public view override returns (address) {
        return _host;
    }

    /**
     * @notice Returns the address of the underlying ERC20 token
     * @return The underlying token address
     */
    function underlying() public view returns (address) {
        return _underlying;
    }

    /**
     * @notice Returns the CallDispatcher address
     * @return The dispatcher contract address
     */
    function dispatcher() public view returns (address) {
        return _dispatcher;
    }

    /**
     * @notice Returns whether the underlying token is WETH
     * @return True if the underlying token is WETH
     */
    function isWeth() public view returns (bool) {
        return _isWeth;
    }

    /**
     * @notice Returns the module ID of the peer on a given chain
     * @param chainId The chain identifier
     * @return The module ID of the peer on the specified chain
     */
    function supportedChain(bytes calldata chainId) public view returns (bytes memory) {
        return _supportedChains[chainId];
    }

    /**
     * @notice Configures the host, dispatcher, and underlying token addresses
     * @dev Only callable by the contract owner
     * @param options The configuration parameters
     */
    function configure(WrappedConfigOptions calldata options) external onlyOwner {
        if (_host == address(0)) {
            _host = options.host;
        }
        _dispatcher = options.dispatcher;
        _underlying = options.underlying;
        _isWeth = options.isWeth;
    }

    /**
     * @notice Registers a supported chain and its corresponding wrapper contract address
     * @dev Only callable by the contract owner
     * @param chainId The chain identifier (e.g., StateMachine.evm(1))
     * @param moduleId The module ID of the peer on the specified chain
     */
    function addChain(bytes calldata chainId, bytes calldata moduleId) external onlyOwner {
        _supportedChains[chainId] = moduleId;
    }

    /**
     * @notice Removes a chain from the supported set
     * @dev Only callable by the contract owner. After removal, transfers to/from this chain will revert.
     * @param chainId The chain identifier to remove
     */
    function removeChain(bytes calldata chainId) external onlyOwner {
        delete _supportedChains[chainId];
    }

    /**
     * @notice Pauses all cross-chain operations (send and receive)
     * @dev Only callable by the contract owner
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @notice Unpauses all cross-chain operations
     * @dev Only callable by the contract owner
     */
    function unpause() external onlyOwner {
        _unpause();
    }

    /**
     * @notice Locks underlying tokens and dispatches a cross-chain transfer message
     * @dev If `msg.value >= params.amount`, wraps native tokens via the underlying's WETH
     * deposit function (reverts if the underlying is not WETH). The remainder of msg.value
     * after wrapping is forwarded as native payment for dispatch fees.
     *
     * If `msg.value < params.amount`, locks ERC20 tokens via safeTransferFrom and pays
     * dispatch fees in the host's fee token (pulled from msg.sender).
     *
     * @param params The send parameters including destination, recipient, amount, and optional calldata
     */
    function send(HyperFungibleToken.SendParams calldata params) external payable whenNotPaused {
        bytes memory dest = _supportedChains[params.dest];
        if (dest.length == 0) revert UnsupportedChain();

        uint256 msgValue = msg.value;
        if (_isWeth) {
            msgValue = msgValue - params.amount;
            IWETH(_underlying).deposit{value: params.amount}();
        } else {
            IERC20(_underlying).safeTransferFrom(msg.sender, address(this), params.amount);
        }

        bytes memory from = abi.encodePacked(msg.sender);
        bytes memory body = abi.encode(HyperFungibleToken.Message({
            from: from,
            to: params.to,
            amount: params.amount,
            data: params.data
        }));

        DispatchPost memory request = DispatchPost({
            dest: params.dest,
            to: dest,
            body: body,
            timeout: params.timeout,
            fee: params.relayerFee,
            payer: msg.sender
        });

        bytes32 commitment;
        if (msgValue > 0) {
            commitment = IDispatcher(_host).dispatch{value: msgValue}(request);
        } else {
            commitment = dispatchWithFeeToken(request, msg.sender);
        }

        emit Sent(msg.sender, params.to, string(params.dest), params.amount, commitment);
    }

    /**
     * @notice Handles incoming cross-chain token transfer messages
     * @dev Called by the ISMP host when a POST request is received. Verifies the source
     * address matches the configured contract for that chain, then transfers the underlying
     * ERC20 to the recipient. If calldata is present, executes it via the CallDispatcher.
     * @param incoming The incoming POST request containing the token transfer message
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost whenNotPaused {
        PostRequest calldata request = incoming.request;

        bytes memory expectedSource = _supportedChains[request.source];
        if (expectedSource.length == 0) revert UnsupportedChain();
        if (keccak256(request.from) != keccak256(expectedSource)) revert UnauthorizedSource();

        HyperFungibleToken.Message memory message = abi.decode(request.body, (HyperFungibleToken.Message));
        address beneficiary = _toAddr(message.to);

        if (_isWeth) {
            IWETH(_underlying).withdraw(message.amount);
            (bool sent,) = beneficiary.call{value: message.amount}("");
            if (!sent) revert TransferFailed();
        } else {
            IERC20(_underlying).safeTransfer(beneficiary, message.amount);
        }

        if (message.data.length > 0) {
            ICallDispatcher(_dispatcher).dispatch(message.data);
        }

        emit Received(message.from, beneficiary, string(request.source), message.amount);
    }

    /**
     * @notice Handles timeout of a previously dispatched cross-chain transfer
     * @dev Called by the ISMP host when a sent message times out without being delivered.
     * Attempts to unwrap WETH and refund native tokens. If that fails (underlying is not
     * WETH or native transfer rejected), falls back to transferring the underlying ERC20.
     * @param request The timed-out POST request
     */
    function onPostRequestTimeout(PostRequest calldata request) external override onlyHost whenNotPaused {
        HyperFungibleToken.Message memory message = abi.decode(request.body, (HyperFungibleToken.Message));
        address refundee = _toAddr(message.from);

        if (_isWeth) {
            IWETH(_underlying).withdraw(message.amount);
            (bool sent,) = refundee.call{value: message.amount}("");
            if (!sent) revert TransferFailed();
        } else {
            IERC20(_underlying).safeTransfer(refundee, message.amount);
        }

        emit Refunded(refundee, message.amount);
    }

    /// @notice Accepts native ETH transfers, required for receiving ETH from WETH.withdraw()
    receive() external payable {}

    /// @notice Extracts an address from the first 20 bytes of a bytes memory value
    function _toAddr(bytes memory b) internal pure returns (address addr) {
        if (b.length < 20) revert InvalidAddress(b.length);
        assembly {
            addr := mload(add(b, 20))
        }
    }

    /// @notice ERC165 interface detection
    function supportsInterface(bytes4 interfaceId) public view override(ERC165) returns (bool) {
        return interfaceId == type(IWrappedHyperFungibleToken).interfaceId || super.supportsInterface(interfaceId);
    }
}
