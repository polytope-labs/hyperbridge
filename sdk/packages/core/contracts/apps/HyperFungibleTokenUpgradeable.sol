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

import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {ERC165Upgradeable} from "@openzeppelin/contracts-upgradeable/utils/introspection/ERC165Upgradeable.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import {IHyperFungibleToken} from "../interfaces/IHyperFungibleToken.sol";
import {PostRequest} from "../libraries/Message.sol";
import {DispatchPost, IDispatcher} from "../interfaces/IDispatcher.sol";
import {IncomingPostRequest, PostRequestTimeout} from "../interfaces/IApp.sol";
import {ICallDispatcher} from "../interfaces/ICallDispatcher.sol";
import {HyperApp} from "./HyperApp.sol";

/**
 * @title HyperFungibleTokenUpgradeable
 * @author Polytope Labs (hello@polytope.technology)
 * @notice Upgradeable cross-chain fungible token that is its own bridge application. Each token deployment
 * is its own bridge application — no shared custody pool. Burns tokens on the source
 * chain and mints on the destination chain.
 *
 * @dev Inherits ERC20Upgradeable for token logic, HyperApp for cross-chain message handling,
 * and OwnableUpgradeable for chain configuration. The owner configures which chains this token can
 * communicate with and the address of the corresponding deployment on each chain.
 *
 * Supports optional calldata execution on the destination chain via CallDispatcher,
 * enabling composable cross-chain interactions (e.g., transfer-and-swap).
 */
contract HyperFungibleTokenUpgradeable is
    Initializable,
    ERC20Upgradeable,
    ERC165Upgradeable,
    HyperApp,
    OwnableUpgradeable,
    PausableUpgradeable
{
    using SafeERC20 for IERC20;

    /**
     * @title SendParams
     * @notice Parameters for initiating a cross-chain token transfer
     */
    struct SendParams {
        /// @notice Destination chain identifier (e.g., StateMachine.evm(1))
        bytes dest;
        /// @notice Recipient account on the destination chain
        bytes to;
        /// @notice Amount of tokens to send
        uint256 amount;
        /// @notice Timeout duration in seconds for the cross-chain message
        uint64 timeout;
        /// @notice Fee paid to relayers for message delivery
        uint256 relayerFee;
        /**
         * @notice Optional calldata to execute on the destination chain via CallDispatcher.
         * Should be an abi-encoded Call[] array.
         */
        bytes data;
    }

    /**
     * @title ConfigOptions
     * @notice Configuration parameters for HyperFungibleTokenUpgradeable
     */
    struct ConfigOptions {
        /// @notice Address of the ISMP host contract on this chain
        address host;
        /// @notice Address of the CallDispatcher contract for executing calldata on receive
        address dispatcher;
    }

    /**
     * @title Message
     * @notice The cross-chain message body for token transfers
     */
    struct Message {
        /// @notice The original sender on the source chain (used for timeout refunds)
        bytes from;
        /// @notice The recipient on the destination chain
        bytes to;
        /// @notice The amount of tokens being transferred
        uint256 amount;
        /// @notice Optional calldata to execute on the destination chain via CallDispatcher
        bytes data;
    }

    /// @notice Thrown when the provided bytes are too short to extract an address
    error InvalidAddress(uint256 length);

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

    /**
     * @notice Maps chain identifiers to the module ID of the peer on that chain.
     * An empty value means the chain is not supported.
     */
    mapping(bytes => bytes) internal _supportedChains;

    /**
     * @notice Emitted when tokens are burned and a cross-chain transfer is dispatched
     * @param from The sender on the source chain
     * @param to The recipient on the destination chain
     * @param dest The destination chain identifier
     * @param amount The amount of tokens sent
     * @param commitment The ISMP request commitment hash for tracking
     */
    event Sent(address from, bytes to, string dest, uint256 amount, bytes32 commitment);

    /**
     * @notice Emitted when tokens are minted from an incoming cross-chain transfer
     * @param from The original sender on the source chain
     * @param to The recipient on this chain
     * @param source The source chain identifier
     * @param amount The amount of tokens minted
     */
    event Received(bytes from, address to, string source, uint256 amount);

    /**
     * @notice Emitted when tokens are refunded after a cross-chain transfer timeout
     * @param to The original sender being refunded
     * @param amount The amount of tokens refunded
     */
    event Refunded(address to, uint256 amount);

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /**
     * @notice Initializes the token with a name, symbol, and owner
     * @param name The name of the token
     * @param symbol The symbol of the token
     * @param initialOwner The address that will own this contract
     */
    function initialize(string memory name, string memory symbol, address initialOwner) public virtual initializer {
        __HyperFungibleToken_init(name, symbol, initialOwner);
    }

    /**
     * @notice Initializes the inherited upgradeable modules for derived contracts
     * @param name The name of the token
     * @param symbol The symbol of the token
     * @param initialOwner The address that will own this contract
     */
    function __HyperFungibleToken_init(string memory name, string memory symbol, address initialOwner)
        internal
        virtual
        onlyInitializing
    {
        __ERC20_init(name, symbol);
        __ERC165_init();
        __Ownable_init(initialOwner);
        __Pausable_init();
    }

    /**
     * @notice Returns the ISMP host address for this chain
     * @return The host contract address
     */
    function host() public view override returns (address) {
        return _host;
    }

    /**
     * @notice Returns the CallDispatcher address
     * @return The dispatcher contract address
     */
    function dispatcher() public view returns (address) {
        return _dispatcher;
    }

    /**
     * @notice Returns the token contract address for a given chain
     * @param chainId The chain identifier
     * @return The address of the token contract on the specified chain
     */
    function supportedChain(bytes calldata chainId) public view returns (bytes memory) {
        return _supportedChains[chainId];
    }

    /**
     * @notice Configures the host and dispatcher addresses
     * @dev Only callable by the contract owner
     * @param options The configuration parameters containing host and dispatcher addresses
     */
    function configure(ConfigOptions calldata options) external onlyOwner {
        if (_host == address(0)) {
            _host = options.host;
        }
        _dispatcher = options.dispatcher;
    }

    /**
     * @notice Registers a supported chain and its corresponding token contract address
     * @dev Only callable by the contract owner. The address is the token contract on that chain.
     * @param chainId The chain identifier (e.g., StateMachine.evm(1))
     * @param moduleId The module ID of the peer on the specified chain (8 bytes for pallet, 20 bytes for EVM contract)
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
     * @notice Returns the fee in native currency for sending a cross-chain transfer.
     * @param params The send parameters
     * @return The fee amount in native currency
     */
    function quote(SendParams calldata params) public view returns (uint256) {
        return quote(_buildDispatchPost(params));
    }

    /**
     * @dev Builds the DispatchPost from SendParams.
     */
    function _buildDispatchPost(SendParams calldata params) internal view returns (DispatchPost memory) {
        bytes memory dest = _supportedChains[params.dest];
        if (dest.length == 0) revert UnsupportedChain();

        bytes memory body = abi.encode(
            Message({from: abi.encodePacked(msg.sender), to: params.to, amount: params.amount, data: params.data})
        );

        return DispatchPost({
            dest: params.dest,
            to: dest,
            body: body,
            timeout: params.timeout,
            fee: params.relayerFee,
            payer: msg.sender
        });
    }

    /**
     * @dev Burns `params.amount` from the caller and sends an ISMP POST request to the
     * destination chain. Fees can be paid in native tokens (via msg.value) or in the
     * host's fee token (pulled from msg.sender).
     * @param params The send parameters including destination, recipient, amount, and optional calldata
     */
    function send(SendParams calldata params) external payable whenNotPaused {
        _burn(msg.sender, params.amount);
        DispatchPost memory request = _buildDispatchPost(params);

        bytes32 commitment;
        if (msg.value > 0) {
            commitment = IDispatcher(_host).dispatch{value: msg.value}(request);
        } else {
            commitment = dispatchWithFeeToken(request);
        }

        emit Sent({
            from: msg.sender,
            to: params.to,
            dest: string(params.dest),
            amount: params.amount,
            commitment: commitment
        });
    }

    /**
     * @notice Handles incoming cross-chain token transfer messages
     * @dev Called by the ISMP host when a POST request is received. Verifies the source
     * address matches the configured contract for that chain, then mints tokens to the
     * recipient. If calldata is present, executes it via the CallDispatcher.
     * @param incoming The incoming POST request containing the token transfer message
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost whenNotPaused {
        PostRequest calldata request = incoming.request;

        bytes memory expectedSource = _supportedChains[request.source];
        if (expectedSource.length == 0) revert UnsupportedChain();
        if (keccak256(request.from) != keccak256(expectedSource)) revert UnauthorizedSource();

        Message memory message = abi.decode(request.body, (Message));
        address beneficiary = _toAddr(message.to);
        _mint(beneficiary, message.amount);

        if (message.data.length > 0) {
            ICallDispatcher(_dispatcher).dispatch(message.data);
        }

        emit Received({from: message.from, to: beneficiary, source: string(request.source), amount: message.amount});
    }

    /**
     * @notice Handles timeout of a previously dispatched cross-chain transfer
     * @dev Called by the ISMP host when a sent message times out without being delivered.
     * Re-mints the burned tokens back to the original sender as a refund.
     * @param incoming The timed-out POST request and the relayer that submitted the timeout proof
     */
    function onPostRequestTimeout(PostRequestTimeout memory incoming) external override onlyHost whenNotPaused {
        Message memory message = abi.decode(incoming.request.body, (Message));
        address refundee = _toAddr(message.from);
        _mint(refundee, message.amount);
        emit Refunded({to: refundee, amount: message.amount});
    }

    /// @notice Extracts an address from the first 20 bytes of a bytes memory value
    function _toAddr(bytes memory b) internal pure returns (address addr) {
        if (b.length != 20) revert InvalidAddress(b.length);
        // casting to 'bytes20' is safe because we already checked length
        // forge-lint: disable-next-line(unsafe-typecast)
        return address(bytes20(b));
    }

    /// @notice ERC165 interface detection
    function supportsInterface(bytes4 interfaceId) public view virtual override(ERC165Upgradeable) returns (bool) {
        return interfaceId == type(IHyperFungibleToken).interfaceId || super.supportsInterface(interfaceId);
    }
}
