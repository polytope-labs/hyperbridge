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

import {Context} from "openzeppelin/utils/Context.sol";
import {Math} from "openzeppelin/utils/math/Math.sol";
import {Strings} from "openzeppelin/utils/Strings.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin/token/ERC20/utils/SafeERC20.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";

import {IIsmpModule, IncomingPostRequest, IncomingPostResponse, IncomingGetResponse} from "ismp/IIsmpModule.sol";
import {DispatchPost, DispatchPostResponse, DispatchGet} from "ismp/IDispatcher.sol";
import {IIsmpHost, FeeMetadata, ResponseReceipt} from "ismp/IIsmpHost.sol";
import {StateCommitment, StateMachineHeight} from "ismp/IConsensusClient.sol";
import {IHandler} from "ismp/IHandler.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout, Message} from "ismp/Message.sol";

// The IsmpHost parameters
struct HostParams {
    // default timeout in seconds for requests.
    uint256 defaultTimeout;
    // cost of cross-chain requests in the fee token per byte
    uint256 perByteFee;
    // The fee token contract. This will typically be DAI.
    // but we allow it to be configurable to prevent future regrets.
    address feeToken;
    // admin account, this only has the rights to freeze, or unfreeze the bridge
    address admin;
    // Ismp request/response handler
    address handler;
    // the authorized host manager contract
    address hostManager;
    // unstaking period
    uint256 unStakingPeriod;
    // minimum challenge period in seconds;
    uint256 challengePeriod;
    // consensus client contract
    address consensusClient;
    // whitelisted state machines
    uint256[] stateMachineWhitelist;
    // white list of fishermen accounts
    address[] fishermen;
    // state machine identifier for hyperbridge
    bytes hyperbridge;
}

// The host manager interface. This provides methods for modifying the host's params or withdrawing bridge revenue.
// Can only be called used by the HostManager module.
interface IHostManager {
    /**
     * @dev Updates IsmpHost params
     * @param params new IsmpHost params
     */
    function updateHostParams(HostParams memory params) external;

    /**
     * @dev withdraws bridge revenue to the given address
     * @param params, the parameters for withdrawal
     */
    function withdraw(WithdrawParams memory params) external;
}

// Withdraw parameters
struct WithdrawParams {
    // The beneficiary address
    address beneficiary;
    // the amount to be disbursed
    uint256 amount;
}

/// IsmpHost implementation for Evm hosts. Refer to the official ISMP specification.
/// https://docs.hyperbridge.network/protocol/ismp
abstract contract EvmHost is IIsmpHost, IHostManager, Context {
    using Bytes for bytes;
    using Message for PostResponse;
    using Message for PostRequest;
    using Message for GetRequest;

    // commitment of all outgoing requests and amount put up for relayers.
    mapping(bytes32 => FeeMetadata) private _requestCommitments;

    // commitment of all outgoing responses and amount put up for relayers.
    mapping(bytes32 => FeeMetadata) private _responseCommitments;

    // commitment of all incoming requests and who delivered them.
    mapping(bytes32 => address) private _requestReceipts;

    // commitment of all incoming responses and who delivered them.
    // maps the request commitment to a receipt object
    mapping(bytes32 => ResponseReceipt) private _responseReceipts;

    // commitment of all incoming requests that have been responded to
    mapping(bytes32 => bool) private _responded;

    // mapping of state machine identifier to latest known height to state commitment
    // (stateMachineId => (blockHeight => StateCommitment))
    mapping(uint256 => mapping(uint256 => StateCommitment)) private _stateCommitments;

    // mapping of state machine identifier to latest known height to update time
    // (stateMachineId => (blockHeight => timestamp))
    mapping(uint256 => mapping(uint256 => uint256)) private _stateCommitmentsUpdateTime;

    // mapping of state machine identifier to latest known height
    // (stateMachineId => blockHeight)
    mapping(uint256 => uint256) private _latestStateMachineHeight;

    // mapping of all known fishermen accounts
    mapping(address => bool) private _fishermen;

    // mapping of state machine identifier to height vetoed to fisherman
    // useful for rewarding fishermen on hyperbridge
    // (stateMachineId => (blockHeight => fisherman))
    mapping(uint256 => mapping(uint256 => address)) private _vetoes;

    // Parameters for the host
    HostParams private _hostParams;

    // monotonically increasing nonce for outgoing requests
    uint256 private _nonce;

    // emergency shutdown button, only the admin can do this
    bool private _frozen;

    // current verified state of the consensus client;
    bytes private _consensusState;

    // timestamp for when the consensus was most recently updated
    uint256 private _consensusUpdateTimestamp;

    // Emitted when an incoming POST request is handled
    event PostRequestHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing POST request timeout is handled, `dest` refers
    // to the destination for the request
    event PostRequestTimeoutHandled(bytes32 commitment, bytes dest);

    // Emitted when an incoming POST response is handled
    event PostResponseHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing POST response timeout is handled, `dest` refers
    // to the destination for the response
    event PostResponseTimeoutHandled(bytes32 commitment, bytes dest);

    // Emitted when an outgoing GET request is handled
    event GetRequestHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing GET request timeout is handled, `dest` refers
    // to the destination for the request
    event GetRequestTimeoutHandled(bytes32 commitment, bytes dest);

    // Emitted when new heights are finalized
    event StateMachineUpdated(bytes stateMachineId, uint256 height);

    // Emitted when a state commitment is vetoed by a fisherman
    event StateCommitmentVetoed(
        bytes stateMachineId, uint256 height, StateCommitment stateCommitment, address fisherman
    );

    // Emitted when a new POST request is dispatched
    event PostRequestEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes to,
        uint256 indexed nonce,
        uint256 timeoutTimestamp,
        bytes data,
        uint256 fee
    );

    // Emitted when a new POST response is dispatched
    event PostResponseEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes to,
        uint256 indexed nonce,
        uint256 timeoutTimestamp,
        bytes data,
        bytes response,
        uint256 resTimeoutTimestamp,
        uint256 fee
    );

    // Emitted when a new GET request is dispatched
    event GetRequestEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes[] keys,
        uint256 indexed nonce,
        uint256 height,
        uint256 timeoutTimestamp
    );

    // Emitted when a POST or GET request is funded
    event RequestFunded(bytes32 commitment, uint256 newFee);

    // Emitted when a POST response is funded
    event PostResponseFunded(bytes32 commitment, uint256 newFee);

    // Emitted when the host has now been frozen
    event HostFrozen();

    // Emitted when the host is unfrozen
    event HostUnfrozen();

    // Emitted when the host params is updated
    event HostParamsUpdated(HostParams oldParams, HostParams newParams);

    // Account is unauthorized to perform requested action
    error UnauthorizedAccount();
    // Provided address didn't fit address type size
    error InvalidAddressLength();
    // Provided request was unknown
    error UnknownRequest();
    // Provided response was unknown
    error UnknownResponse();
    // Action breaks protocol invariants and is therefore unauthorized
    error UnauthorizedAction();
    // Application is attempting to respond to a request it did not receive
    error UnauthorizedResponse();
    // Response has already been provided for this request
    error DuplicateResponse();
    // Cannot exceed max fishermen count
    error MaxFishermanCountExceeded(uint256 provided);
    // Host manager address was zero or not a contract
    error InvalidHostManagerAddress();

    // only permits fishermen
    modifier onlyFishermen() {
        if (!_fishermen[_msgSender()]) {
            revert UnauthorizedAccount();
        }
        _;
    }

    // restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (_msgSender() != caller) revert UnauthorizedAction();
        _;
    }

    constructor(HostParams memory params) {
        updateHostParamsInternal(params);
        _consensusUpdateTimestamp = block.timestamp;
    }

    /**
     * @return the host admin
     */
    function admin() external view returns (address) {
        return _hostParams.admin;
    }

    /**
     * @return the host state machine id
     */
    function host() public view virtual returns (bytes memory);

    /**
     * @return the mainnet evm chainId for this host
     */
    function chainId() public virtual returns (uint256);

    /**
     * @return the address of the fee token ERC-20 contract on this state machine
     */
    function feeToken() public view returns (address) {
        return _hostParams.feeToken;
    }

    /**
     * @return the per-byte fee for outgoing requests.
     */
    function perByteFee() external view returns (uint256) {
        return _hostParams.perByteFee;
    }

    /**
     * @return the host timestamp
     */
    function timestamp() external view returns (uint256) {
        return block.timestamp;
    }

    /**
     * @return the `frozen` status
     */
    function frozen() external view returns (bool) {
        return _frozen;
    }

    /**
     * @return the `HostParams`
     */
    function hostParams() external view returns (HostParams memory) {
        return _hostParams;
    }

    /**
     * @return the state machine identifier for the connected hyperbridge instance
     */
    function hyperbridge() external view returns (bytes memory) {
        return _hostParams.hyperbridge;
    }

    /**
     * @param height - state machine height
     * @return the state commitment at `height`
     */
    function stateMachineCommitment(StateMachineHeight memory height) external view returns (StateCommitment memory) {
        return _stateCommitments[height.stateMachineId][height.height];
    }

    /**
     * @param height - state machine height
     * @return the state machine update time at `height`
     */
    function stateMachineCommitmentUpdateTime(StateMachineHeight memory height) external view returns (uint256) {
        return _stateCommitmentsUpdateTime[height.stateMachineId][height.height];
    }

    /**
     * @dev Should return a handle to the consensus client based on the id
     * @return the consensus client contract
     */
    function consensusClient() external view returns (address) {
        return _hostParams.consensusClient;
    }

    /**
     * @return the last updated time of the consensus client
     */
    function consensusUpdateTime() external view returns (uint256) {
        return _consensusUpdateTimestamp;
    }

    /**
     * @return the state of the consensus client
     */
    function consensusState() external view returns (bytes memory) {
        return _consensusState;
    }

    /**
     * @return the challenge period
     */
    function challengePeriod() external view returns (uint256) {
        return _hostParams.challengePeriod;
    }

    /**
     * @return the latest state machine height for the given stateMachineId. If it returns 0, the state machine is unsupported.
     */
    function latestStateMachineHeight(uint256 id) external view returns (uint256) {
        return _latestStateMachineHeight[id];
    }

    /**
     * @return the unstaking period
     */
    function unStakingPeriod() external view returns (uint256) {
        return _hostParams.unStakingPeriod;
    }

    /**
     * @param commitment - commitment to the request
     * @return existence status of an incoming request commitment
     */
    function requestReceipts(bytes32 commitment) external view returns (address) {
        return _requestReceipts[commitment];
    }

    /**
     * @param commitment - commitment to the response
     * @return existence status of an incoming response commitment
     */
    function responseReceipts(bytes32 commitment) external view returns (ResponseReceipt memory) {
        return _responseReceipts[commitment];
    }

    /**
     * @param commitment - commitment to the request
     * @return existence status of an outgoing request commitment
     */
    function requestCommitments(bytes32 commitment) external view returns (FeeMetadata memory) {
        return _requestCommitments[commitment];
    }

    /**
     * @param commitment - commitment to the response
     * @return existence status of an outgoing response commitment
     */
    function responseCommitments(bytes32 commitment) external view returns (FeeMetadata memory) {
        return _responseCommitments[commitment];
    }

    /**
     * @dev Updates the HostParams, can only be called by cross-chain governance
     * @param params, the new host params.
     */
    function updateHostParams(HostParams memory params) external restrict(_hostParams.hostManager) {
        updateHostParamsInternal(params);
    }

    /**
     * @dev Updates the HostParams
     * @param params, the new host params. Can only be called by admin on testnets.
     */
    function setHostParamsAdmin(HostParams memory params) public restrict(_hostParams.admin) {
        if (chainId() == block.chainid) {
            revert UnauthorizedAction();
        }

        uint256 whitelistLength = params.stateMachineWhitelist.length;
        for (uint256 i = 0; i < whitelistLength; ++i) {
            delete _latestStateMachineHeight[params.stateMachineWhitelist[i]];
        }
        updateHostParamsInternal(params);
    }

    /**
     * @dev Updates the HostParams
     * @param params, the new host params.
     */
    function updateHostParamsInternal(HostParams memory params) private {
        // check if the provided host manager is a contract
        if (params.hostManager == address(0) || address(params.hostManager).code.length == 0) {
            revert InvalidHostManagerAddress();
        }

        // we can only have a maximum of 100 fishermen
        uint256 newFishermenLength = params.fishermen.length;
        if (newFishermenLength > 100) {
            revert MaxFishermanCountExceeded(newFishermenLength);
        }

        // delete old fishermen
        uint256 fishermenLength = _hostParams.fishermen.length;
        for (uint256 i = 0; i < fishermenLength; ++i) {
            delete _fishermen[_hostParams.fishermen[i]];
        }

        // safe to emit here because invariants have already been checked
        // and don't want to store a temp variable for the old params
        emit HostParamsUpdated({oldParams: _hostParams, newParams: params});

        _hostParams = params;

        // add new fishermen if any
        for (uint256 i = 0; i < newFishermenLength; ++i) {
            _fishermen[params.fishermen[i]] = true;
        }

        // add whitelisted state machines
        uint256 whitelistLength = params.stateMachineWhitelist.length;
        for (uint256 i = 0; i < whitelistLength; ++i) {
            // create if it doesn't already exist
            if (_latestStateMachineHeight[params.stateMachineWhitelist[i]] == 0) {
                _latestStateMachineHeight[params.stateMachineWhitelist[i]] = 1;
            }
        }
    }

    /**
     * @dev withdraws host revenue to the given address, can only be called by cross-chain governance
     * @param params, the parameters for withdrawal
     */
    function withdraw(WithdrawParams memory params) external restrict(_hostParams.hostManager) {
        SafeERC20.safeTransfer(IERC20(feeToken()), params.beneficiary, params.amount);
    }

    /**
     * @dev Store the serialized consensus state, alongside relevant metadata
     */
    function storeConsensusState(bytes memory state) external restrict(_hostParams.handler) {
        _consensusState = state;
        _consensusUpdateTimestamp = block.timestamp;
    }

    /**
     * @dev Store the state commitment at given state height alongside relevant metadata. Assumes the state commitment is of the latest height.
     */
    function storeStateMachineCommitment(StateMachineHeight memory height, StateCommitment memory commitment)
        external
        restrict(_hostParams.handler)
    {
        _stateCommitments[height.stateMachineId][height.height] = commitment;
        _stateCommitmentsUpdateTime[height.stateMachineId][height.height] = block.timestamp;
        _latestStateMachineHeight[height.stateMachineId] = height.height;

        emit StateMachineUpdated({stateMachineId: stateMachineId(height.stateMachineId), height: height.height});
    }

    /**
     * @dev Delete the state commitment at given state height.
     */
    function deleteStateMachineCommitment(StateMachineHeight memory height, address fisherman)
        external
        restrict(_hostParams.handler)
    {
        deleteStateMachineCommitmentInternal(height, fisherman);
    }

    /**
     * @dev A fisherman has determined that some [`StateCommitment`]
     *  (which is ideally still in it's challenge period)
     *  is infact fraudulent and misrepresentative of the state
     *  changes at the provided height. This allows them to veto the state commitment.
     *  At the moment, they aren't required to provide any proofs for this.
     */
    function vetoStateCommitment(StateMachineHeight memory height) public onlyFishermen {
        deleteStateMachineCommitmentInternal(height, _msgSender());
    }

    /**
     * @dev Delete the state commitment at given state height.
     */
    function deleteStateMachineCommitmentInternal(StateMachineHeight memory height, address fisherman) private {
        StateCommitment memory stateCommitment = _stateCommitments[height.stateMachineId][height.height];
        delete _stateCommitments[height.stateMachineId][height.height];
        delete _stateCommitmentsUpdateTime[height.stateMachineId][height.height];
        // technically any state commitment can be vetoed, safety check that it's the latest before resetting it.
        if (_latestStateMachineHeight[height.stateMachineId] == height.height) {
            _latestStateMachineHeight[height.stateMachineId] = 1;
        }

        // track the fisherman responsible for rewards on hyperbridge through state proofs
        _vetoes[height.stateMachineId][height.height] = fisherman;

        emit StateCommitmentVetoed({
            stateMachineId: stateMachineId(height.stateMachineId),
            stateCommitment: stateCommitment,
            height: height.height,
            fisherman: fisherman
        });
    }

    /**
     * @dev Get the state machine id for a parachain
     */
    function stateMachineId(uint256 id) public view returns (bytes memory) {
        bytes memory hyperbridgeId = _hostParams.hyperbridge;
        uint256 offset = hyperbridgeId.length - 4;
        return bytes.concat(hyperbridgeId.substr(0, offset), bytes(Strings.toString(id)));
    }

    /**
     * @dev set the new state of the bridge
     * @param newState new state
     */
    function setFrozenState(bool newState) public restrict(_hostParams.admin) {
        _frozen = newState;

        if (newState) {
            emit HostFrozen();
        } else {
            emit HostUnfrozen();
        }
    }

    /**
     * @dev sets the initial consensus state
     * @param state initial consensus state
     */
    function setConsensusState(bytes memory state, StateMachineHeight memory height, StateCommitment memory commitment)
        public
        restrict(_hostParams.admin)
    {
        // if we're on mainnet, then consensus state can only be initialized once
        // and updated subsequently through consensus proofs
        require(chainId() == block.chainid ? _consensusState.equals(new bytes(0)) : true, "Unauthorized action");

        _consensusState = state;
        _consensusUpdateTimestamp = block.timestamp;

        _stateCommitments[height.stateMachineId][height.height] = commitment;
        _stateCommitmentsUpdateTime[height.stateMachineId][height.height] = block.timestamp;
        _latestStateMachineHeight[height.stateMachineId] = height.height;
    }

    /**
     * @dev Dispatch an incoming POST request to destination module
     * @param request - post request
     */
    function dispatchIncoming(PostRequest memory request, address relayer) external restrict(_hostParams.handler) {
        address destination = _bytesToAddress(request.to);
        uint256 size;
        assembly {
            size := extcodesize(destination)
        }
        if (size == 0) {
            // instead of reverting the entire batch, early return here.
            return;
        }

        // replay protection
        bytes32 commitment = request.hash();
        _requestReceipts[commitment] = relayer;

        (bool success,) = address(destination).call(
            abi.encodeWithSelector(IIsmpModule.onAccept.selector, IncomingPostRequest(request, relayer))
        );

        if (!success) {
            // so that it can be retried
            delete _requestReceipts[commitment];
            return;
        }
        emit PostRequestHandled({commitment: commitment, relayer: relayer});
    }

    /**
     * @dev Dispatch an incoming POST response to source module
     * @param response - post response
     */
    function dispatchIncoming(PostResponse memory response, address relayer) external restrict(_hostParams.handler) {
        address origin = _bytesToAddress(response.request.from);

        // replay protection
        bytes32 requestCommitment = response.request.hash();
        bytes32 responseCommitment = response.hash();
        _responseReceipts[requestCommitment] =
            ResponseReceipt({relayer: relayer, responseCommitment: responseCommitment});

        (bool success,) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onPostResponse.selector, IncomingPostResponse(response, relayer))
        );

        if (!success) {
            // so that it can be retried
            delete _responseReceipts[requestCommitment];
            return;
        }
        emit PostResponseHandled({commitment: responseCommitment, relayer: relayer});
    }

    /**
     * @dev Dispatch an incoming GET response to source module
     * @param response - get response
     */
    function dispatchIncoming(GetResponse memory response, address relayer) external restrict(_hostParams.handler) {
        address origin = _bytesToAddress(response.request.from);

        // replay protection
        bytes32 commitment = response.request.hash();
        // don't commit the full response object, it's unused.
        _responseReceipts[commitment] = ResponseReceipt({relayer: relayer, responseCommitment: bytes32(0)});

        (bool success,) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onGetResponse.selector, IncomingGetResponse(response, relayer))
        );

        if (!success) {
            // so that it can be retried
            delete _responseReceipts[commitment];
            return;
        }

        emit PostResponseHandled({commitment: commitment, relayer: relayer});
    }

    /**
     * @dev Dispatch an incoming GET timeout to the source module
     * @param request - get request
     */
    function dispatchIncoming(GetRequest memory request, FeeMetadata memory meta, bytes32 commitment)
        external
        restrict(_hostParams.handler)
    {
        address origin = _bytesToAddress(request.from);

        // replay protection, delete memory of this request
        delete _requestCommitments[commitment];
        (bool success,) = address(origin).call(abi.encodeWithSelector(IIsmpModule.onGetTimeout.selector, request));

        if (!success) {
            // so that it can be retried
            _requestCommitments[commitment] = meta;
            return;
        }

        emit GetRequestTimeoutHandled({commitment: commitment, dest: request.dest});
    }

    /**
     * @dev Dispatch an incoming POST timeout to the source module
     * @param request - post timeout
     */
    function dispatchIncoming(PostRequest memory request, FeeMetadata memory meta, bytes32 commitment)
        external
        restrict(_hostParams.handler)
    {
        address origin = _bytesToAddress(request.from);

        // replay protection, delete memory of this request
        delete _requestCommitments[commitment];
        (bool success,) =
            address(origin).call(abi.encodeWithSelector(IIsmpModule.onPostRequestTimeout.selector, request));

        if (!success) {
            // so that it can be retried
            _requestCommitments[commitment] = meta;
            return;
        }

        if (meta.fee != 0) {
            SafeERC20.safeTransfer(IERC20(feeToken()), meta.sender, meta.fee);
        }
        emit PostRequestTimeoutHandled({commitment: commitment, dest: request.dest});
    }

    /**
     * @dev Dispatch an incoming POST response timeout to the source module
     * @param response - timed-out post response
     */
    function dispatchIncoming(PostResponse memory response, FeeMetadata memory meta, bytes32 commitment)
        external
        restrict(_hostParams.handler)
    {
        address origin = _bytesToAddress(response.request.to);

        // replay protection, delete memory of this response
        bytes32 reqCommitment = response.request.hash();
        delete _responseCommitments[commitment];
        delete _responded[reqCommitment];
        (bool success,) =
            address(origin).call(abi.encodeWithSelector(IIsmpModule.onPostResponseTimeout.selector, response));

        if (!success) {
            // so that it can be retried
            _responseCommitments[commitment] = meta;
            _responded[reqCommitment] = true;
            return;
        }

        if (meta.fee != 0) {
            // refund relayer fee
            SafeERC20.safeTransfer(IERC20(feeToken()), meta.sender, meta.fee);
        }
        emit PostResponseTimeoutHandled({commitment: commitment, dest: response.request.source});
    }

    /**
     * @dev Dispatch a POST request to the hyperbridge
     * @param post - post request
     */
    function dispatch(DispatchPost memory post) external returns (bytes32 commitment) {
        uint256 fee = (_hostParams.perByteFee * post.body.length) + post.fee;
        SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), fee);

        // adjust the timeout
        uint64 timeout = post.timeout == 0
            ? 0
            : uint64(this.timestamp()) + uint64(Math.max(_hostParams.defaultTimeout, post.timeout));
        PostRequest memory request = PostRequest({
            source: host(),
            dest: post.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            to: post.to,
            timeoutTimestamp: timeout,
            body: post.body
        });

        // make the commitment
        commitment = request.hash();
        _requestCommitments[commitment] = FeeMetadata({sender: post.payer, fee: post.fee});
        emit PostRequestEvent(
            request.source,
            request.dest,
            request.from,
            abi.encodePacked(request.to),
            request.nonce,
            request.timeoutTimestamp,
            request.body,
            post.fee
        );
    }

    /**
     * @dev Dispatch a GET request to the hyperbridge
     * @param get - get request
     */
    function dispatch(DispatchGet memory get) external returns (bytes32 commitment) {
        if (get.fee != 0) {
            SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), get.fee);
        }

        // adjust the timeout
        uint64 timeout =
            get.timeout == 0 ? 0 : uint64(this.timestamp()) + uint64(Math.max(_hostParams.defaultTimeout, get.timeout));

        GetRequest memory request = GetRequest({
            source: host(),
            dest: get.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            timeoutTimestamp: timeout,
            keys: get.keys,
            height: get.height
        });

        // make the commitment
        commitment = request.hash();
        _requestCommitments[commitment] = FeeMetadata({sender: get.sender, fee: get.fee});
        emit GetRequestEvent(
            request.source,
            request.dest,
            request.from,
            request.keys,
            request.nonce,
            request.height,
            request.timeoutTimestamp
        );
    }

    /**
     * @dev Dispatch a POST response to the hyperbridge
     * @param post - post response
     */
    function dispatch(DispatchPostResponse memory post) external returns (bytes32 commitment) {
        bytes32 receipt = post.request.hash();

        // known request?
        if (_requestReceipts[receipt] == address(0)) {
            revert UnknownRequest();
        }

        // check that the authorized application is issuing this response
        if (_bytesToAddress(post.request.to) != _msgSender()) {
            revert UnauthorizedResponse();
        }

        // check that request has not already been responed to
        if (_responded[receipt]) {
            revert DuplicateResponse();
        }

        // collect fees
        uint256 fee = (_hostParams.perByteFee * post.response.length) + post.fee;
        SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), fee);

        // adjust the timeout
        uint64 timeout = post.timeout == 0
            ? 0
            : uint64(this.timestamp()) + uint64(Math.max(_hostParams.defaultTimeout, post.timeout));

        PostResponse memory response =
            PostResponse({request: post.request, response: post.response, timeoutTimestamp: timeout});
        commitment = response.hash();

        FeeMetadata memory meta = FeeMetadata({fee: post.fee, sender: post.payer});
        _responseCommitments[commitment] = meta;
        _responded[receipt] = true;

        emit PostResponseEvent(
            response.request.source,
            response.request.dest,
            response.request.from,
            abi.encodePacked(response.request.to),
            response.request.nonce,
            response.request.timeoutTimestamp,
            response.request.body,
            response.response,
            response.timeoutTimestamp,
            meta.fee // sigh solidity
        );
    }

    /**
     * @dev Increase the relayer fee for a previously dispatched request.
     * This is provided for use only on pending requests, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * If called on an already delivered request, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The request commitment
     */
    function fundRequest(bytes32 commitment, uint256 amount) public {
        FeeMetadata memory metadata = _requestCommitments[commitment];

        if (metadata.sender == address(0)) {
            revert UnknownRequest();
        }
        SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), amount);

        metadata.fee += amount;
        _requestCommitments[commitment] = metadata;

        emit RequestFunded({commitment: commitment, newFee: metadata.fee});
    }

    /**
     * @dev Increase the relayer fee for a previously dispatched response.
     * This is provided for use only on pending responses, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * If called on an already delivered response, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The response commitment
     */
    function fundResponse(bytes32 commitment, uint256 amount) public {
        FeeMetadata memory metadata = _responseCommitments[commitment];

        if (metadata.sender == address(0)) {
            revert UnknownResponse();
        }
        SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), amount);

        metadata.fee += amount;
        _responseCommitments[commitment] = metadata;

        emit PostResponseFunded({commitment: commitment, newFee: metadata.fee});
    }

    /**
     * @dev Get next available nonce for outgoing requests.
     */
    function _nextNonce() private returns (uint256) {
        uint256 _nonce_copy = _nonce;

        unchecked {
            ++_nonce;
        }

        return _nonce_copy;
    }

    /**
     * @dev Converts bytes to address.
     * @param _bytes bytes value to be converted
     * @return addr returns the address
     */
    function _bytesToAddress(bytes memory _bytes) private pure returns (address addr) {
        if (_bytes.length != 20) {
            revert InvalidAddressLength();
        }
        assembly {
            addr := mload(add(_bytes, 20))
        }
    }
}
