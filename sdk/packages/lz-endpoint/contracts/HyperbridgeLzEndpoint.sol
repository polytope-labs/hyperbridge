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

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";

import {PostRequest} from "@hyperbridge/core/libraries/Message.sol";
import {DispatchPost, IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IncomingPostRequest} from "@hyperbridge/core/interfaces/IApp.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";

import {
    ILayerZeroEndpointV2,
    MessagingParams,
    MessagingReceipt,
    MessagingFee,
    Origin
} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {ILayerZeroReceiver} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroReceiver.sol";
import {ILayerZeroComposer} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroComposer.sol";
import {SetConfigParam} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLibManager.sol";

/**
 * @title HyperbridgeLzEndpoint
 * @author Polytope Labs (hello@polytope.technology)
 * @notice A LayerZero V2 endpoint adapter that routes messages through Hyperbridge's ISMP
 * protocol. Existing OFTs can point to this contract as their LayerZero endpoint to use
 * Hyperbridge for cross-chain transport without code changes.
 *
 * @dev Implements `ILayerZeroEndpointV2` for OApp compatibility and `HyperApp` for ISMP
 * message handling. Assumes the same contract address on all chains (CREATE2 deployment).
 *
 * EID-to-StateMachine mapping is owner-configured via `setEidMapping()`.
 */
contract HyperbridgeLzEndpoint is HyperApp, Ownable, Pausable, ILayerZeroEndpointV2 {
    /// @notice Thrown when the destination eid has no configured state machine mapping
    error UnknownEid(uint32 eid);

    /// @notice Thrown when an incoming message has an unexpected source
    error UnknownSource();

    /// @notice Thrown when an inbound nonce is out of order
    error InvalidNonce(uint64 expected, uint64 got);

    /// @notice Thrown when a compose message doesn't match the queued hash
    error InvalidCompose();

    /// @notice Address of the ISMP host contract
    address internal _host;

    /// @notice The local endpoint ID for this chain
    uint32 internal _eid;

    /// @notice Maps LZ endpoint ID → ISMP state machine identifier
    mapping(uint32 => bytes) internal _eidToStateMachine;

    /// @notice Maps ISMP state machine identifier (keccak hash) → LZ endpoint ID
    mapping(bytes32 => uint32) internal _stateMachineToEid;

    /// @notice Outbound nonce per (sender, dstEid, receiver)
    mapping(address => mapping(uint32 => mapping(bytes32 => uint64))) internal _outboundNonce;

    /// @notice Inbound nonce per (receiver, srcEid, sender)
    mapping(address => mapping(uint32 => mapping(bytes32 => uint64))) internal _inboundNonce;

    /// @notice Per-destination relayer fee (in feeToken units). Defaults to $0.30.
    mapping(uint32 => uint256) internal _relayerFees;

    /// @notice Default relayer fee used when no per-destination fee is configured
    uint256 internal _defaultRelayerFee;

    /// @notice Compose message queue: keccak256(from, to, guid, index) => keccak256(message)
    mapping(bytes32 => bytes32) internal _composeQueue;

    constructor() Ownable(msg.sender) {}

    // ==================== Configuration ====================

    /**
     * @notice Configures the ISMP host address and local endpoint ID
     * @param hostAddr The ISMP host contract address
     * @param localEid The LayerZero endpoint ID for this chain
     */
    function setHost(address hostAddr, uint32 localEid) external onlyOwner {
        _host = hostAddr;
        _eid = localEid;

        // Set default relayer fee to $0.30 based on feeToken decimals
        address feeToken = IDispatcher(hostAddr).feeToken();
        uint8 decimals = IERC20Metadata(feeToken).decimals();
        _defaultRelayerFee = (3 * 10 ** decimals) / 10; // 0.30
    }

    /**
     * @notice Sets the relayer fee for a specific destination chain
     * @param dstEid The destination endpoint ID
     * @param fee The relayer fee in feeToken units
     */
    function setRelayerFee(uint32 dstEid, uint256 fee) external onlyOwner {
        _relayerFees[dstEid] = fee;
    }

    /**
     * @notice Sets the default relayer fee used when no per-destination fee is configured
     * @param fee The default relayer fee in feeToken units
     */
    function setDefaultRelayerFee(uint256 fee) external onlyOwner {
        _defaultRelayerFee = fee;
    }

    /**
     * @notice Returns the relayer fee for a destination, falling back to the default
     * @param dstEid The destination endpoint ID
     */
    function relayerFee(uint32 dstEid) public view returns (uint256) {
        uint256 fee = _relayerFees[dstEid];
        return fee > 0 ? fee : _defaultRelayerFee;
    }

    /**
     * @notice Registers a bidirectional mapping between a LZ eid and an ISMP state machine ID
     * @param lzEid The LayerZero endpoint ID
     * @param stateMachineId The ISMP state machine identifier (e.g., StateMachine.evm(1))
     */
    function setEidMapping(uint32 lzEid, bytes calldata stateMachineId) external onlyOwner {
        _eidToStateMachine[lzEid] = stateMachineId;
        _stateMachineToEid[keccak256(stateMachineId)] = lzEid;
    }

    /// @inheritdoc HyperApp
    function host() public view override returns (address) {
        return _host;
    }

    /**
     * @notice Returns the ISMP state machine identifier for a given LZ endpoint ID
     * @param lzEid The LayerZero endpoint ID
     * @return The ISMP state machine identifier
     */
    function eidMapping(uint32 lzEid) public view returns (bytes memory) {
        return _eidToStateMachine[lzEid];
    }

    /**
     * @notice Returns the LZ endpoint ID for a given ISMP state machine identifier
     * @param stateMachineId The ISMP state machine identifier
     * @return The LZ endpoint ID (0 if not configured)
     */
    function eidFor(bytes calldata stateMachineId) public view returns (uint32) {
        return _stateMachineToEid[keccak256(stateMachineId)];
    }

    /**
     * @notice Returns the default relayer fee
     * @return The default relayer fee in feeToken units
     */
    function defaultRelayerFee() public view returns (uint256) {
        return _defaultRelayerFee;
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

    // ==================== Core Endpoint Functions ====================

    /// @inheritdoc ILayerZeroEndpointV2
    function send(
        MessagingParams calldata _params,
        address /* _refundAddress */
    ) external payable override whenNotPaused returns (MessagingReceipt memory) {
        bytes memory dest = _eidToStateMachine[_params.dstEid];
        if (dest.length == 0) revert UnknownEid(_params.dstEid);

        // Track nonce
        uint64 nonce = ++_outboundNonce[msg.sender][_params.dstEid][_params.receiver];

        // Compute globally unique identifier
        bytes32 guid = keccak256(
            abi.encodePacked(nonce, _eid, bytes32(uint256(uint160(msg.sender))), _params.dstEid, _params.receiver)
        );

        // Encode the LZ message into the ISMP body
        bytes memory body = abi.encode(
            guid,
            _eid, // srcEid
            bytes32(uint256(uint160(msg.sender))), // sender
            nonce,
            _params.receiver, // receiver OApp on dest
            _params.message
        );

        DispatchPost memory request = DispatchPost({
            dest: dest,
            to: abi.encodePacked(address(this)),
            body: body,
            timeout: 0,
            fee: relayerFee(_params.dstEid),
            payer: address(this)
        });

        if (msg.value > 0) {
            IDispatcher(_host).dispatch{value: msg.value}(request);
        } else {
            // Fee tokens already transferred to this contract by OFT's _payLzToken
            dispatchWithFeeToken(request, address(this));
        }

        return MessagingReceipt({
            guid: guid,
            nonce: nonce,
            fee: MessagingFee({nativeFee: msg.value, lzTokenFee: 0})
        });
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function quote(
        MessagingParams calldata _params,
        address _sender
    ) external view override returns (MessagingFee memory) {
        bytes memory dest = _eidToStateMachine[_params.dstEid];
        if (dest.length == 0) revert UnknownEid(_params.dstEid);

        // Build equivalent body to estimate fee
        bytes memory body = abi.encode(
            bytes32(0), uint32(0), bytes32(0), uint64(0), _params.receiver, _params.message
        );

        DispatchPost memory request = DispatchPost({
            dest: dest,
            to: abi.encodePacked(address(this)),
            body: body,
            timeout: 0,
            fee: relayerFee(_params.dstEid),
            payer: _sender
        });

        if (_params.payInLzToken) {
            uint256 feeTokenAmount = quote(request);
            return MessagingFee({nativeFee: 0, lzTokenFee: feeTokenAmount});
        } else {
            uint256 nativeFee = quoteNative(request);
            return MessagingFee({nativeFee: nativeFee, lzTokenFee: 0});
        }
    }

    // ==================== ISMP Callbacks ====================

    /**
     * @notice Handles incoming cross-chain messages from ISMP and delivers them to OApps
     * @dev Decodes the ISMP body into LZ message components, validates the nonce,
     * and calls `lzReceive` on the destination OApp.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost whenNotPaused {
        PostRequest calldata request = incoming.request;

        // Verify source is this adapter on another chain
        if (keccak256(request.from) != keccak256(abi.encodePacked(address(this)))) revert UnknownSource();

        // Decode the LZ message from the ISMP body
        (
            bytes32 guid,
            uint32 srcEid,
            bytes32 sender,
            uint64 nonce,
            bytes32 receiver,
            bytes memory message
        ) = abi.decode(request.body, (bytes32, uint32, bytes32, uint64, bytes32, bytes));

        // Validate source eid matches the ISMP source chain
        uint32 expectedEid = _stateMachineToEid[keccak256(request.source)];
        if (expectedEid != srcEid) revert UnknownSource();

        // Validate and increment nonce
        address receiverAddr = address(uint160(uint256(receiver)));
        uint64 expectedNonce = _inboundNonce[receiverAddr][srcEid][sender] + 1;
        if (nonce != expectedNonce) revert InvalidNonce(expectedNonce, nonce);
        _inboundNonce[receiverAddr][srcEid][sender] = nonce;

        // Deliver to the OApp
        Origin memory origin = Origin({srcEid: srcEid, sender: sender, nonce: nonce});
        ILayerZeroReceiver(receiverAddr).lzReceive(origin, guid, message, address(0), "");
    }

    /**
     * @notice Handles ISMP request timeouts
     * @dev LZ messages don't have a timeout concept — messages are retried, not expired.
     * This is a no-op since we dispatch with timeout=0 (no expiry).
     */
    function onPostRequestTimeout(PostRequest memory) external override onlyHost {}

    // ==================== LZ Endpoint Stubs ====================

    /// @inheritdoc ILayerZeroEndpointV2
    function lzReceive(
        Origin calldata,
        address,
        bytes32,
        bytes calldata,
        bytes calldata
    ) external payable override {
        // This is called by the endpoint to deliver messages to OApps.
        // In our adapter, we call lzReceive on OApps directly from onAccept.
        // This function should not be called externally.
        revert("HyperbridgeLzEndpoint: use ISMP for delivery");
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function verify(Origin calldata, address, bytes32) external pure override {
        // ISMP handles all verification
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function verifiable(Origin calldata, address) external pure override returns (bool) {
        return true;
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function initializable(Origin calldata, address) external pure override returns (bool) {
        return true;
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function clear(address, Origin calldata, bytes32, bytes calldata) external pure override {}

    /// @inheritdoc ILayerZeroEndpointV2
    function setLzToken(address) external pure override {}

    /// @inheritdoc ILayerZeroEndpointV2
    function lzToken() external view override returns (address) {
        return IDispatcher(_host).feeToken();
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function nativeToken() external pure override returns (address) {
        return address(0);
    }

    /// @inheritdoc ILayerZeroEndpointV2
    function setDelegate(address) external pure override {}

    // ==================== IMessagingChannel ====================

    function eid() external view override returns (uint32) {
        return _eid;
    }

    function skip(address, uint32, bytes32, uint64) external pure override {}

    function nilify(address, uint32, bytes32, uint64, bytes32) external pure override {}

    function burn(address, uint32, bytes32, uint64, bytes32) external pure override {}

    function nextGuid(
        address _sender,
        uint32 _dstEid,
        bytes32 _receiver
    ) external view override returns (bytes32) {
        uint64 nonce = _outboundNonce[_sender][_dstEid][_receiver] + 1;
        return keccak256(
            abi.encodePacked(nonce, _eid, bytes32(uint256(uint160(_sender))), _dstEid, _receiver)
        );
    }

    function inboundNonce(
        address _receiver,
        uint32 _srcEid,
        bytes32 _sender
    ) external view override returns (uint64) {
        return _inboundNonce[_receiver][_srcEid][_sender];
    }

    function outboundNonce(
        address _sender,
        uint32 _dstEid,
        bytes32 _receiver
    ) external view override returns (uint64) {
        return _outboundNonce[_sender][_dstEid][_receiver];
    }

    function inboundPayloadHash(address, uint32, bytes32, uint64) external pure override returns (bytes32) {
        return bytes32(0);
    }

    function lazyInboundNonce(
        address _receiver,
        uint32 _srcEid,
        bytes32 _sender
    ) external view override returns (uint64) {
        return _inboundNonce[_receiver][_srcEid][_sender];
    }

    // ==================== IMessagingComposer ====================

    /**
     * @notice Returns the hash of a queued compose message
     * @param _from The OApp that initiated the compose
     * @param _to The target ILayerZeroComposer
     * @param _guid The message guid
     * @param _index The compose message index
     * @return messageHash The keccak256 hash of the queued message, or bytes32(0) if not queued
     */
    function composeQueue(
        address _from,
        address _to,
        bytes32 _guid,
        uint16 _index
    ) external view override returns (bytes32) {
        return _composeQueue[keccak256(abi.encodePacked(_from, _to, _guid, _index))];
    }

    /**
     * @notice Queues a compose message for later execution via lzCompose
     * @dev Called by OApps during lzReceive. Stores the message hash for verification.
     * @param _to The address of the ILayerZeroComposer to receive the compose call
     * @param _guid The unique identifier of the message
     * @param _index The index of the composed message
     * @param _message The composed message payload
     */
    function sendCompose(address _to, bytes32 _guid, uint16 _index, bytes calldata _message) external override {
        bytes32 key = keccak256(abi.encodePacked(msg.sender, _to, _guid, _index));
        _composeQueue[key] = keccak256(_message);
        emit ComposeSent(msg.sender, _to, _guid, _index, _message);
    }

    /**
     * @notice Executes a queued compose message
     * @dev Can be called by anyone (typically a relayer/executor). Verifies the message
     * hash matches the queue, deletes it, then calls lzCompose on the target composer.
     * @param _from The OApp that initiated the compose
     * @param _to The target ILayerZeroComposer
     * @param _guid The message guid
     * @param _index The compose message index
     * @param _message The composed message payload (must match the queued hash)
     * @param _extraData Additional data passed to the composer
     */
    function lzCompose(
        address _from,
        address _to,
        bytes32 _guid,
        uint16 _index,
        bytes calldata _message,
        bytes calldata _extraData
    ) external payable override {
        bytes32 key = keccak256(abi.encodePacked(_from, _to, _guid, _index));
        bytes32 expectedHash = _composeQueue[key];
        if (expectedHash == bytes32(0) || expectedHash != keccak256(_message)) revert InvalidCompose();

        delete _composeQueue[key];

        ILayerZeroComposer(_to).lzCompose{value: msg.value}(_from, _guid, _message, msg.sender, _extraData);
        emit ComposeDelivered(_from, _to, _guid, _index);
    }

    // ==================== IMessagingContext ====================

    function isSendingMessage() external pure override returns (bool) {
        return false;
    }

    function getSendContext() external pure override returns (uint32, address) {
        return (0, address(0));
    }

    // ==================== IMessageLibManager (stubs) ====================

    function registerLibrary(address) external pure override {}
    function isRegisteredLibrary(address) external pure override returns (bool) { return true; }
    function getRegisteredLibraries() external pure override returns (address[] memory) {
        return new address[](0);
    }
    function setDefaultSendLibrary(uint32, address) external pure override {}
    function defaultSendLibrary(uint32) external view override returns (address) { return address(this); }
    function setDefaultReceiveLibrary(uint32, address, uint256) external pure override {}
    function defaultReceiveLibrary(uint32) external view override returns (address) { return address(this); }
    function setDefaultReceiveLibraryTimeout(uint32, address, uint256) external pure override {}
    function defaultReceiveLibraryTimeout(uint32) external pure override returns (address, uint256) {
        return (address(0), 0);
    }
    function isSupportedEid(uint32 _lzEid) external view override returns (bool) {
        return _eidToStateMachine[_lzEid].length > 0;
    }
    function isValidReceiveLibrary(address, uint32, address) external pure override returns (bool) { return true; }
    function setSendLibrary(address, uint32, address) external pure override {}
    function getSendLibrary(address, uint32) external view override returns (address) { return address(this); }
    function isDefaultSendLibrary(address, uint32) external pure override returns (bool) { return true; }
    function setReceiveLibrary(address, uint32, address, uint256) external pure override {}
    function getReceiveLibrary(address, uint32) external view override returns (address, bool) {
        return (address(this), true);
    }
    function setReceiveLibraryTimeout(address, uint32, address, uint256) external pure override {}
    function receiveLibraryTimeout(address, uint32) external pure override returns (address, uint256) {
        return (address(0), 0);
    }
    function setConfig(address, address, SetConfigParam[] calldata) external pure override {}
    function getConfig(address, address, uint32, uint32) external pure override returns (bytes memory) {
        return "";
    }

}
