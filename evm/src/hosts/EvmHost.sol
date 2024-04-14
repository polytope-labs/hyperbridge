// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {Context} from "openzeppelin/utils/Context.sol";
import {Math} from "openzeppelin/utils/math/Math.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
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
    // current verified state of the consensus client;
    bytes consensusState;
    // timestamp for when the consensus was most recently updated
    uint256 consensusUpdateTimestamp;
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
    // (stateMachineId => (blockHeight => timestamp))
    mapping(uint256 => uint256) private _latestStateMachineHeight;

    // mapping of all known fishermen accounts
    // (stateMachineId => (blockHeight => timestamp))
    mapping(address => bool) private _fishermen;

    // Parameters for the host
    HostParams private _hostParams;

    // monotonically increasing nonce for outgoing requests
    uint256 private _nonce;

    // emergency shutdown button, only the admin can do this
    bool private _frozen;

    // Emitted when an incoming POST request is handled
    event PostRequestHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing POST request timeout is handled
    event PostRequestTimeoutHandled(bytes32 commitment);

    // Emitted when an incoming POST response is handled
    event PostResponseHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing POST timeout response is handled
    event PostResponseTimeoutHandled(bytes32 commitment);

    // Emitted when an outgoing GET request is handled
    event GetRequestHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing GET request timeout is handled
    event GetRequestTimeoutHandled(bytes32 commitment);

    // Emitted when new heights are finalized
    event StateMachineUpdated(uint256 stateMachineId, uint256 height);

    // Emitted when a state commitment is vetoed by a fisherman
    event StateCommitmentVetoed(uint256 stateMachineId, uint256 height, address fisherman);

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

    // only permits fishermen
    modifier onlyFishermen() {
        require(_fishermen[_msgSender()], "EvmHost: Account is not in the fishermen set");
        _;
    }

    // only permits the admin
    modifier onlyAdmin() {
        require(_msgSender() == _hostParams.admin, "EvmHost: Account is not the admin");
        _;
    }

    // only permits the IHandler contract
    modifier onlyHandler() {
        require(_msgSender() == address(_hostParams.handler), "EvmHost: Account is not the handler");
        _;
    }

    // only permits the HostManager contract
    modifier onlyManager() {
        require(_msgSender() == _hostParams.hostManager, "EvmHost: Account is not the Manager contract");
        _;
    }

    constructor(HostParams memory params) {
        _hostParams = params;

        // add fishermen if any
        uint256 newFishermenLength = params.fishermen.length;
        for (uint256 i = 0; i < newFishermenLength; i++) {
            _fishermen[params.fishermen[i]] = true;
        }

        // add whitelisted state machines
        uint256 whitelistLength = params.stateMachineWhitelist.length;
        for (uint256 i = 0; i < whitelistLength; i++) {
            _latestStateMachineHeight[params.stateMachineWhitelist[i]] = 1;
        }
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
        return _hostParams.consensusUpdateTimestamp;
    }

    /**
     * @return the state of the consensus client
     */
    function consensusState() external view returns (bytes memory) {
        return _hostParams.consensusState;
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
    function latestStateMachineHeight(uint256 stateMachineId) external view returns (uint256) {
        return _latestStateMachineHeight[stateMachineId];
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
    function updateHostParams(HostParams memory params) external onlyManager {
        updateHostParamsInternal(params);
    }

    /**
     * @dev Updates the HostParams
     * @param params, the new host params. Can only be called by admin on testnets.
     */
    function setHostParamsAdmin(HostParams memory params) public onlyAdmin {
        require(chainId() != block.chainid, "Cannot set params on mainnet");

        updateHostParamsInternal(params);
    }

    /**
     * @dev Updates the HostParams
     * @param params, the new host params.
     */
    function updateHostParamsInternal(HostParams memory params) private {
        // delete old fishermen
        uint256 fishermenLength = _hostParams.fishermen.length;
        for (uint256 i = 0; i < fishermenLength; i++) {
            delete _fishermen[_hostParams.fishermen[i]];
        }
        _hostParams = params;

        // add new fishermen if any
        uint256 newFishermenLength = params.fishermen.length;
        for (uint256 i = 0; i < newFishermenLength; i++) {
            _fishermen[params.fishermen[i]] = true;
        }
    }

    /**
     * @dev withdraws host revenue to the given address,  can only be called by cross-chain governance
     * @param params, the parameters for withdrawal
     */
    function withdraw(WithdrawParams memory params) external onlyManager {
        require(IERC20(feeToken()).transfer(params.beneficiary, params.amount), "Host has an insufficient balance");
    }

    /**
     * @dev Store the serialized consensus state, alongside relevant metadata
     */
    function storeConsensusState(bytes memory state) external onlyHandler {
        _hostParams.consensusState = state;
        _hostParams.consensusUpdateTimestamp = block.timestamp;
    }

    /**
     * @dev Store the state commitment at given state height alongside relevant metadata. Assumes the state commitment is of the latest height.
     */
    function storeStateMachineCommitment(StateMachineHeight memory height, StateCommitment memory commitment)
        external
        onlyHandler
    {
        _stateCommitments[height.stateMachineId][height.height] = commitment;
        _stateCommitmentsUpdateTime[height.stateMachineId][height.height] = block.timestamp;
        _latestStateMachineHeight[height.stateMachineId] = height.height;

        emit StateMachineUpdated({stateMachineId: height.stateMachineId, height: height.height});
    }

    /**
     * @dev Delete the state commitment at given state height.
     */
    function deleteStateMachineCommitment(StateMachineHeight memory height, address fisherman) external onlyHandler {
        deleteStateMachineCommitmentInternal(height, fisherman);
    }

    /**
     * @dev Delete the state commitment at given state height.
     */
    function deleteStateMachineCommitmentInternal(StateMachineHeight memory height, address fisherman) private {
        delete _stateCommitments[height.stateMachineId][height.height];
        delete _stateCommitmentsUpdateTime[height.stateMachineId][height.height];
        delete _latestStateMachineHeight[height.stateMachineId];

        emit StateCommitmentVetoed({stateMachineId: height.stateMachineId, height: height.height, fisherman: fisherman});
    }

    /**
     * @dev set the new state of the bridge
     * @param newState new state
     */
    function setFrozenState(bool newState) public onlyAdmin {
        _frozen = newState;
    }

    /**
     * @dev sets the initial consensus state
     * @param state initial consensus state
     */
    function setConsensusState(bytes memory state) public onlyAdmin {
        // if we're on mainnet, then consensus state can only be initialized once.
        // and updated subsequently by either consensus proofs or cross-chain governance
        require(
            chainId() == block.chainid ? _hostParams.consensusState.equals(new bytes(0)) : true, "Unauthorized action"
        );

        _hostParams.consensusState = state;
    }

    /**
     * @dev Dispatch an incoming post request to destination module
     * @param request - post request
     */
    function dispatchIncoming(PostRequest memory request, address relayer) external onlyHandler {
        address destination = _bytesToAddress(request.to);
        uint256 size;
        assembly {
            size := extcodesize(destination)
        }
        if (size == 0) {
            // instead of reverting the entire batch, early return here.
            return;
        }

        (bool success,) = address(destination).call(
            abi.encodeWithSelector(IIsmpModule.onAccept.selector, IncomingPostRequest(request, relayer))
        );

        if (success) {
            bytes32 commitment = request.hash();
            _requestReceipts[commitment] = relayer;

            emit PostRequestHandled({commitment: commitment, relayer: relayer});
        }
    }

    /**
     * @dev Dispatch an incoming post response to source module
     * @param response - post response
     */
    function dispatchIncoming(PostResponse memory response, address relayer) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);
        (bool success,) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onPostResponse.selector, IncomingPostResponse(response, relayer))
        );

        if (success) {
            bytes32 commitment = response.request.hash();
            _responseReceipts[commitment] = ResponseReceipt({relayer: relayer, responseCommitment: response.hash()});

            emit PostResponseHandled({commitment: commitment, relayer: relayer});
        }
    }

    /**
     * @dev Dispatch an incoming get response to source module
     * @param response - get response
     */
    function dispatchIncoming(GetResponse memory response, address relayer) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);
        (bool success,) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onGetResponse.selector, IncomingGetResponse(response, relayer))
        );

        if (success) {
            bytes32 commitment = response.request.hash();
            // don't commit the full response object, it's unused.
            _responseReceipts[commitment] = ResponseReceipt({relayer: relayer, responseCommitment: bytes32(0)});
            emit PostResponseHandled({commitment: commitment, relayer: relayer});
        }
    }

    /**
     * @dev Dispatch an incoming get timeout to the source module
     * @param request - get request
     */
    function dispatchIncoming(GetRequest memory request, FeeMetadata memory meta, bytes32 commitment)
        external
        onlyHandler
    {
        address origin = _bytesToAddress(request.from);
        (bool success,) = address(origin).call(abi.encodeWithSelector(IIsmpModule.onGetTimeout.selector, request));

        if (success) {
            // delete memory of this request
            delete _requestCommitments[commitment];

            if (meta.fee > 0) {
                // refund relayer fee
                IERC20(feeToken()).transfer(meta.sender, meta.fee);
            }

            emit GetRequestTimeoutHandled({commitment: commitment});
        }
    }

    /**
     * @dev Dispatch an incoming post timeout to the source module
     * @param request - post timeout
     */
    function dispatchIncoming(PostRequest memory request, FeeMetadata memory meta, bytes32 commitment)
        external
        onlyHandler
    {
        address origin = _bytesToAddress(request.from);
        (bool success,) =
            address(origin).call(abi.encodeWithSelector(IIsmpModule.onPostRequestTimeout.selector, request));

        if (success) {
            // delete memory of this request
            delete _requestCommitments[commitment];

            if (meta.fee > 0) {
                // refund relayer fee
                IERC20(feeToken()).transfer(meta.sender, meta.fee);
            }

            emit PostRequestTimeoutHandled({commitment: commitment});
        }
    }

    /**
     * @dev Dispatch an incoming post response timeout to the source module
     * @param response - timed-out post response
     */
    function dispatchIncoming(PostResponse memory response, FeeMetadata memory meta, bytes32 commitment)
        external
        onlyHandler
    {
        address origin = _bytesToAddress(response.request.to);
        (bool success,) =
            address(origin).call(abi.encodeWithSelector(IIsmpModule.onPostResponseTimeout.selector, response));

        if (success) {
            // delete memory of this response
            delete _responseCommitments[commitment];
            delete _responded[response.request.hash()];

            if (meta.fee > 0) {
                // refund relayer fee
                IERC20(feeToken()).transfer(meta.sender, meta.fee);
            }

            emit PostResponseTimeoutHandled({commitment: commitment});
        }
    }

    /**
     * @dev Dispatch a POST request to the hyperbridge
     * @param post - post request
     */
    function dispatch(DispatchPost memory post) external returns (bytes32 commitment) {
        uint256 fee = (_hostParams.perByteFee * post.body.length) + post.fee;
        IERC20(feeToken()).transferFrom(_msgSender(), address(this), fee);

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
     * @dev Dispatch a get request to the hyperbridge
     * @param get - get request
     */
    function dispatch(DispatchGet memory get) external returns (bytes32 commitment) {
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
        _requestCommitments[commitment] = FeeMetadata({sender: get.sender, fee: 0});
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
     * @dev Dispatch a response to the hyperbridge
     * @param post - post response
     */
    function dispatch(DispatchPostResponse memory post) external returns (bytes32 commitment) {
        bytes32 receipt = post.request.hash();

        // known request?
        require(_requestReceipts[receipt] != address(0), "EvmHost: Unknown request");

        // check that the authorized application is issuing this response
        require(_bytesToAddress(post.request.to) == _msgSender(), "EvmHost: Unauthorized Response");

        // check that request has not already been responed to
        require(!_responded[receipt], "EvmHost: Duplicate Response");

        uint256 fee = (_hostParams.perByteFee * post.response.length) + post.fee;
        IERC20(feeToken()).transferFrom(_msgSender(), address(this), fee);

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

        require(metadata.sender != address(0), "Unknown request");
        require(metadata.sender != _msgSender(), "User can only fund own requests");
        IERC20(feeToken()).transferFrom(_msgSender(), address(this), amount);

        metadata.fee += amount;
        _requestCommitments[commitment] = metadata;
    }

    /**
     * @dev A fisherman has determined that some [`StateCommitment`]
     *  (which is ideally still in it's challenge period)
     *  is infact fraudulent and misrepresentative of the state
     *  changes at the provided height. This allows them to veto the state commitment.
     *  They aren't required to provide any proofs for this.
     */
    function vetoStateCommitment(StateMachineHeight memory height) public onlyFishermen {
        deleteStateMachineCommitmentInternal(height, _msgSender());
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
        require(_bytes.length >= 20, "Invalid address length");
        assembly {
            addr := mload(add(_bytes, 20))
        }
    }
}
