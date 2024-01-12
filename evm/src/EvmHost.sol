// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "openzeppelin/utils/Context.sol";
import "openzeppelin/utils/math/Math.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";

import {IIsmpModule} from "ismp/IIsmpModule.sol";
import {IIsmpHost, FeeMetadata} from "ismp/IIsmpHost.sol";
import {StateCommitment, StateMachineHeight} from "ismp/IConsensusClient.sol";
import {IHandler} from "ismp/IHandler.sol";
import {
    PostRequest,
    PostResponse,
    GetRequest,
    GetResponse,
    PostTimeout,
    DispatchPost,
    DispatchPostResponse,
    DispatchGet,
    Message
} from "ismp/IIsmp.sol";

// The IsmpHost parameters
struct HostParams {
    // default timeout in seconds for requests.
    uint256 defaultTimeout;
    // base fee for GET requests
    uint256 baseGetRequestFee;
    // timestamp for when the consensus was most recently updated
    uint256 lastUpdated;
    // unstaking period
    uint256 unStakingPeriod;
    // minimum challenge period in seconds;
    uint256 challengePeriod;
    // cost of cross-chain requests in $DAI per byte
    uint256 perByteFee;
    // The fee token contract. This will typically be DAI.
    // but we allow it to be configurable to prevent future regrets.
    address feeTokenAddress;
    // consensus client contract
    address consensusClient;
    // admin account, this only has the rights to freeze, or unfreeze the bridge
    address admin;
    // Ismp request/response handler
    address handler;
    // the authorized host manager contract
    address hostManager;
    // current verified state of the consensus client;
    bytes consensusState;
}

// The host manager interface. This provides methods for modifying the host's params or withdrawing bridge revenue.
// Can only be called used by the HostManager module.
interface IHostManager {
    /**
     * @dev Updates IsmpHost params
     * @param params new IsmpHost params
     */
    function setHostParams(HostParams memory params) external;

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

struct ResponseReceipt {
    // commitment of the response object
    bytes32 responseCommitment;
    // address of the relayer responsible for this response delivery
    address relayer;
}

/// Ismp implementation for Evm hosts
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

    // (stateMachineId => (blockHeight => StateCommitment))
    mapping(uint256 => mapping(uint256 => StateCommitment)) private _stateCommitments;

    // (stateMachineId => (blockHeight => timestamp))
    mapping(uint256 => mapping(uint256 => uint256)) private _stateCommitmentsUpdateTime;

    uint256 private _latestStateMachineHeight;

    // Parameters for the host
    HostParams private _hostParams;

    // monotonically increasing nonce for outgoing requests
    uint256 private _nonce;

    // emergency shutdown button, only the admin can do this
    bool private _frozen;

    // Emitted when an incoming POST request is handled
    event PostRequestHandled(bytes32 commitment, address relayer);

    // Emitted when an incoming POST response is handled
    event PostResponseHandled(bytes32 commitment, address relayer);

    // Emitted when an outgoing Get request is handled
    event GetRequestHandled(bytes32 commitment, address relayer);

    event PostRequestEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes to,
        uint256 indexed nonce,
        uint256 timeoutTimestamp,
        bytes data,
        uint256 gaslimit,
        uint256 fee
    );

    event PostResponseEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes to,
        uint256 indexed nonce,
        uint256 timeoutTimestamp,
        bytes data,
        uint256 gaslimit,
        bytes response,
        uint256 resGaslimit,
        uint256 resTimeoutTimestamp,
        uint256 fee
    );

    event GetRequestEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes[] keys,
        uint256 indexed nonce,
        uint256 height,
        uint256 timeoutTimestamp,
        uint256 gaslimit,
        uint256 fee
    );

    modifier onlyAdmin() {
        require(_msgSender() == _hostParams.admin, "EvmHost: Only admin");
        _;
    }

    modifier onlyHandler() {
        require(_msgSender() == address(_hostParams.handler), "EvmHost: Only handler");
        _;
    }

    modifier onlyManager() {
        require(_msgSender() == _hostParams.hostManager, "EvmHost: Only Manager contract");
        _;
    }

    constructor(HostParams memory params) {
        _hostParams = params;
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
    function host() public virtual returns (bytes memory);

    /**
     * @return the address of the DAI ERC-20 contract on this state machine
     */
    function dai() public view returns (address) {
        return _hostParams.feeTokenAddress;
    }

    /**
     * @return the host timestamp
     */
    function timestamp() public view returns (uint256) {
        return block.timestamp;
    }

    /**
     * @return the `frozen` status
     */
    function frozen() public view returns (bool) {
        return _frozen;
    }

    function hostParams() public view returns (HostParams memory) {
        return _hostParams;
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
        return _hostParams.lastUpdated;
    }

    /**
     * @return the state of the consensus client
     */
    function consensusState() external view returns (bytes memory) {
        return _hostParams.consensusState;
    }

    /**
     * @param commitment - commitment to the request
     * @return existence status of an incoming request commitment
     */
    function requestReceipts(bytes32 commitment) external view returns (bool) {
        return _requestReceipts[commitment] != address(0);
    }

    /**
     * @param commitment - commitment to the response
     * @return existence status of an incoming response commitment
     */
    function responseReceipts(bytes32 commitment) external view returns (bool) {
        return _responseReceipts[commitment].relayer != address(0);
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
     * @return the challenge period
     */
    function challengePeriod() external view returns (uint256) {
        return _hostParams.challengePeriod;
    }

    /**
     * @return the latest state machine height
     */
    function latestStateMachineHeight() external view returns (uint256) {
        return _latestStateMachineHeight;
    }

    /**
     * @dev Updates the HostParams
     * @param params, the new host params. If any param is empty, they won't be set.
     * `lastUpdated` param is exempted.
     */
    function setHostParams(HostParams memory params) external onlyManager {
        _hostParams = params;
    }

    /**
     * @dev withdraws bridge revenue to the given address
     * @param params, the parameters for withdrawal
     */
    function withdraw(WithdrawParams memory params) external onlyManager {
        IERC20(dai()).transfer(params.beneficiary, params.amount);
    }

    /**
     * @dev Store an encoded consensus state
     */
    function storeConsensusState(bytes memory state) external onlyHandler {
        _hostParams.consensusState = state;
    }

    /**
     * @dev Store the timestamp when the consensus client was updated
     */
    function storeConsensusUpdateTime(uint256 time) external onlyHandler {
        _hostParams.lastUpdated = time;
    }

    /**
     * @dev Store the latest state machine height
     * @param height State Machine Latest Height
     */
    function storeLatestStateMachineHeight(uint256 height) external onlyHandler {
        _latestStateMachineHeight = height;
    }

    /**
     * @dev Store the commitment at `state height`
     */
    function storeStateMachineCommitment(StateMachineHeight memory height, StateCommitment memory commitment)
        external
        onlyHandler
    {
        _stateCommitments[height.stateMachineId][height.height] = commitment;
    }

    /**
     * @dev Store the timestamp when the state machine was updated
     */
    function storeStateMachineCommitmentUpdateTime(StateMachineHeight memory height, uint256 time)
        external
        onlyHandler
    {
        _stateCommitmentsUpdateTime[height.stateMachineId][height.height] = time;
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
        require(_hostParams.consensusState.equals(new bytes(0)), "Unauthorized action");

        _hostParams.consensusState = state;
    }

    /**
     * @return the unstaking period
     */
    function unStakingPeriod() public view returns (uint256) {
        return _hostParams.unStakingPeriod;
    }

    /**
     * @dev Dispatch an incoming post request to destination module
     * @param request - post request
     */
    function dispatchIncoming(PostRequest memory request) external onlyHandler {
        address destination = _bytesToAddress(request.to);

        // Ideally this would prevent failing requests from poisoning the batch,
        // doesn't work, sigh solidity
        try IIsmpModule(destination).onAccept(request) {} catch {}

        // doesn't matter if it failed, if it failed once, it'll fail again
        bytes32 commitment = request.hash();
        _requestReceipts[commitment] = tx.origin;

        emit PostRequestHandled({commitment: commitment, relayer: tx.origin});
    }

    /**
     * @dev Dispatch an incoming post response to source module
     * @param response - post response
     */
    function dispatchIncoming(PostResponse memory response) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);

        try IIsmpModule(origin).onPostResponse(response) {} catch {}

        bytes32 commitment = response.request.hash();
        _responseReceipts[commitment] = ResponseReceipt({relayer: tx.origin, responseCommitment: response.hash()});

        emit PostResponseHandled({commitment: commitment, relayer: tx.origin});
    }

    /**
     * @dev Dispatch an incoming get response to source module
     * @param response - get response
     */
    function dispatchIncoming(GetResponse memory response) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);

        uint256 fee = 0;
        for (uint256 i = 0; i < response.values.length; i++) {
            fee += (_hostParams.perByteFee * response.values[i].value.length);
        }

        // Relayers pay for Get Responses
        require(IERC20(dai()).transferFrom(tx.origin, address(this), fee), "Insufficient funds");
        try IIsmpModule(origin).onGetResponse(response) {} catch {}

        bytes32 commitment = response.request.hash();
        _responseReceipts[commitment] = ResponseReceipt({relayer: tx.origin, responseCommitment: bytes32(0)});
        // don't commit the full response object because, it's unused.

        emit PostResponseHandled({commitment: commitment, relayer: tx.origin});
    }

    /**
     * @dev Dispatch an incoming get timeout to source module
     * @param request - get request
     */
    function dispatchIncoming(GetRequest memory request, FeeMetadata memory meta, bytes32 commitment)
        external
        onlyHandler
    {
        address origin = _bytesToAddress(request.from);

        try IIsmpModule(origin).onGetTimeout(request) {} catch {}

        // Delete Commitment
        delete _requestCommitments[commitment];

        // refund relayer fee
        IERC20(dai()).transfer(meta.sender, meta.fee);
    }

    /**
     * @dev Dispatch an incoming post timeout to source module
     * @param request - post timeout
     */
    function dispatchIncoming(PostRequest memory request, FeeMetadata memory meta, bytes32 commitment)
        external
        onlyHandler
    {
        address origin = _bytesToAddress(request.from);

        try IIsmpModule(origin).onPostRequestTimeout(request) {} catch {}

        // Delete Commitment
        delete _requestCommitments[commitment];

        // refund relayer fee
        IERC20(dai()).transfer(meta.sender, meta.fee);
    }

    /**
     * @dev Dispatch an incoming post response timeout to source module
     * @param response - timed-out post response
     */
    function dispatchIncoming(PostResponse memory response, FeeMetadata memory meta, bytes32 commitment)
        external
        onlyHandler
    {
        address origin = _bytesToAddress(response.request.to);

        try IIsmpModule(origin).onPostResponseTimeout(response) {} catch {}

        // Delete Commitment
        delete _responseCommitments[commitment];

        // refund relayer fee
        IERC20(dai()).transfer(meta.sender, meta.fee);
    }

    /**
     * @dev Dispatch a POST request to the hyperbridge
     * @param post - post request
     */
    function dispatch(DispatchPost memory post) external {
        // pay your toll to the troll
        uint256 fee = (_hostParams.perByteFee * post.body.length) + post.fee;
        require(IERC20(dai()).transferFrom(tx.origin, address(this), fee), "Insufficient funds");

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
            body: post.body,
            gaslimit: post.gaslimit
        });

        // make the commitment
        _requestCommitments[request.hash()] = FeeMetadata({sender: tx.origin, fee: fee});

        emit PostRequestEvent(
            request.source,
            request.dest,
            request.from,
            abi.encodePacked(request.to),
            request.nonce,
            request.timeoutTimestamp,
            request.body,
            request.gaslimit,
            fee
        );
    }

    /**
     * @dev Dispatch a get request to the hyperbridge
     * @param get - get request
     */
    function dispatch(DispatchGet memory get) external {
        // pay your toll to the troll
        uint256 fee = _hostParams.baseGetRequestFee + get.fee;
        require(IERC20(dai()).transferFrom(tx.origin, address(this), fee), "Insufficient funds");

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
            height: get.height,
            gaslimit: get.gaslimit
        });

        // make the commitment
        _requestCommitments[request.hash()] = FeeMetadata({sender: tx.origin, fee: fee});
        emit GetRequestEvent(
            request.source,
            request.dest,
            request.from,
            request.keys,
            request.nonce,
            request.height,
            request.timeoutTimestamp,
            request.gaslimit,
            fee
        );
    }

    /**
     * @dev Dispatch a response to the hyperbridge
     * @param post - post response
     */
    function dispatch(DispatchPostResponse memory post) external {
        bytes32 receipt = post.request.hash();
        require(_requestReceipts[receipt] != address(0), "EvmHost: Unknown request");

        // validate that the authorized application is issuing this response
        require(_bytesToAddress(post.request.to) == _msgSender(), "EvmHost: Unauthorized Response");

        // check that request has not already been responed to

        // pay your toll to the troll
        uint256 fee = (_hostParams.perByteFee * post.response.length) + post.fee;
        require(IERC20(dai()).transferFrom(tx.origin, address(this), fee), "Insufficient funds");

        // adjust the timeout
        uint64 timeout = post.timeout == 0
            ? 0
            : uint64(this.timestamp()) + uint64(Math.max(_hostParams.defaultTimeout, post.timeout));
        PostResponse memory response = PostResponse({
            request: post.request,
            response: post.response,
            timeoutTimestamp: timeout,
            gaslimit: post.gaslimit
        });
        _responseCommitments[response.hash()] = FeeMetadata({fee: fee, sender: tx.origin});

        emit PostResponseEvent(
            response.request.source,
            response.request.dest,
            response.request.from,
            abi.encodePacked(response.request.to),
            response.request.nonce,
            response.request.timeoutTimestamp,
            response.request.body,
            response.request.gaslimit,
            response.response,
            response.timeoutTimestamp,
            response.gaslimit,
            fee
        );
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
