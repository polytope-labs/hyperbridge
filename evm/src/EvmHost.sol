// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "openzeppelin/utils/Context.sol";
import "openzeppelin/utils/math/Math.sol";

import "ismp/interfaces/IIsmpModule.sol";
import "ismp/interfaces/IIsmpHost.sol";
import "ismp/interfaces/IHandler.sol";

struct HostParams {
    // default timeout in seconds for requests.
    uint256 defaultTimeout;
    // timestamp for when the consensus was most recently updated
    uint256 lastUpdated;
    // unstaking period
    uint256 unStakingPeriod;
    // minimum challenge period in seconds;
    uint256 challengePeriod;
    // consensus client contract
    address consensusClient;
    // admin account, this only has the rights to freeze, or unfreeze the bridge
    address admin;
    // Ismp request/response handler
    address handler;
    // the authorized cross-chain governor contract
    address crosschainGovernor;
    // current verified state of the consensus client;
    bytes consensusState;
}

/// Ismp implementation for Evm hosts
abstract contract EvmHost is IIsmpHost, Context {
    using Bytes for bytes;

    // commitment of all outgoing requests
    mapping(bytes32 => bool) private _requestCommitments;

    // commitment of all outgoing responses
    mapping(bytes32 => bool) private _responseCommitments;

    // commitment of all incoming requests
    mapping(bytes32 => bool) private _requestReceipts;

    // commitment of all incoming responses
    mapping(bytes32 => bool) private _responseReceipts;

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

    event PostResponseEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes to,
        uint256 indexed nonce,
        uint256 timeoutTimestamp,
        bytes data,
        uint256 gaslimit,
        bytes response
    );

    event PostRequestEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes to,
        uint256 indexed nonce,
        uint256 timeoutTimestamp,
        bytes data,
        uint256 gaslimit
    );

    event GetRequestEvent(
        bytes source,
        bytes dest,
        bytes from,
        bytes[] keys,
        uint256 indexed nonce,
        uint256 height,
        uint256 timeoutTimestamp,
        uint256 gaslimit
    );

    modifier onlyAdmin() {
        require(_msgSender() == _hostParams.admin, "EvmHost: Only admin");
        _;
    }

    modifier onlyHandler() {
        require(_msgSender() == address(_hostParams.handler), "EvmHost: Only handler");
        _;
    }

    modifier onlyGovernance() {
        require(_msgSender() == _hostParams.crosschainGovernor, "EvmHost: Only governor contract");
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
        return _requestReceipts[commitment];
    }

    /**
     * @param commitment - commitment to the response
     * @return existence status of an incoming response commitment
     */
    function responseReceipts(bytes32 commitment) external view returns (bool) {
        return _responseReceipts[commitment];
    }

    /**
     * @param commitment - commitment to the request
     * @return existence status of an outgoing request commitment
     */
    function requestCommitments(bytes32 commitment) external view returns (bool) {
        return _requestCommitments[commitment];
    }

    /**
     * @param commitment - commitment to the response
     * @return existence status of an outgoing response commitment
     */
    function responseCommitments(bytes32 commitment) external view returns (bool) {
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
    function latestStateMachineHeight() external returns (uint256) {
        return _latestStateMachineHeight;
    }

    /**
     * @dev Updates bridge params
     * @param params new bridge params
     */
    function setBridgeParams(BridgeParams memory params) external onlyGovernance {
        _hostParams.challengePeriod = params.challengePeriod;
        _hostParams.consensusClient = params.consensus;
        _hostParams.unStakingPeriod = params.unstakingPeriod;

        _hostParams.admin = params.admin;
        _hostParams.defaultTimeout = params.defaultTimeout;
        _hostParams.handler = params.handler;
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
     * @param consensusState initial consensus state
     */
    function setConsensusState(bytes memory consensusState) public onlyAdmin {
        require(_hostParams.consensusState.equals(new bytes(0)), "Unauthorized action");

        _hostParams.consensusState = consensusState;
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
        IIsmpModule(destination).onAccept(request);

        bytes32 commitment = Message.hash(request);
        _requestReceipts[commitment] = true;
    }

    /**
     * @dev Dispatch an incoming post response to source module
     * @param response - post response
     */
    function dispatchIncoming(PostResponse memory response) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);
        IIsmpModule(origin).onPostResponse(response);

        bytes32 commitment = Message.hash(response);
        _responseReceipts[commitment] = true;
    }

    /**
     * @dev Dispatch an incoming get response to source module
     * @param response - get response
     */
    function dispatchIncoming(GetResponse memory response) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);
        IIsmpModule(origin).onGetResponse(response);

        bytes32 commitment = Message.hash(response);
        _responseReceipts[commitment] = true;
    }

    /**
     * @dev Dispatch an incoming get timeout to source module
     * @param request - get request
     */
    function dispatchIncoming(GetRequest memory request) external onlyHandler {
        address origin = _bytesToAddress(request.from);
        IIsmpModule(origin).onGetTimeout(request);

        // Delete Commitment
        bytes32 commitment = Message.hash(request);
        delete _requestCommitments[commitment];
    }

    /**
     * @dev Dispatch an incoming post timeout to source module
     * @param timeout - post timeout
     */
    function dispatchIncoming(PostTimeout memory timeout) external onlyHandler {
        PostRequest memory request = timeout.request;
        address origin = _bytesToAddress(request.from);
        IIsmpModule(origin).onPostTimeout(request);

        // Delete Commitment
        bytes32 commitment = Message.hash(request);
        delete _requestCommitments[commitment];
    }

    /**
     * @dev Dispatch a post request to the hyperbridge
     * @param request - post dispatch request
     */
    function dispatch(DispatchPost memory request) external {
        uint64 timeout = request.timeout == 0
            ? 0
            : uint64(this.timestamp()) + uint64(Math.max(_hostParams.defaultTimeout, request.timeout));
        PostRequest memory _request = PostRequest({
            source: host(),
            dest: request.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            to: request.to,
            timeoutTimestamp: timeout,
            body: request.body,
            gaslimit: request.gaslimit
        });

        // make the commitment
        bytes32 commitment = Message.hash(_request);
        _requestCommitments[commitment] = true;

        emit PostRequestEvent(
            _request.source,
            _request.dest,
            _request.from,
            abi.encodePacked(_request.to),
            _request.nonce,
            _request.timeoutTimestamp,
            _request.body,
            _request.gaslimit
        );
    }

    /**
     * @dev Dispatch a get request to the hyperbridge
     * @param request - get dispatch request
     */
    function dispatch(DispatchGet memory request) external {
        uint64 timeout = uint64(this.timestamp()) + uint64(Math.max(_hostParams.defaultTimeout, request.timeout));
        GetRequest memory _request = GetRequest({
            source: host(),
            dest: request.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            timeoutTimestamp: timeout,
            keys: request.keys,
            height: request.height,
            gaslimit: request.gaslimit
        });

        // make the commitment
        bytes32 commitment = Message.hash(_request);
        _requestCommitments[commitment] = true;

        emit GetRequestEvent(
            _request.source,
            _request.dest,
            _request.from,
            _request.keys,
            _request.nonce,
            request.height,
            _request.timeoutTimestamp,
            request.gaslimit
        );
    }

    /**
     * @dev Dispatch a response to the hyperbridge
     * @param response - post response
     */
    function dispatch(PostResponse memory response) external {
        bytes32 receipt = Message.hash(response.request);
        require(_requestReceipts[receipt], "EvmHost: unknown request");

        bytes32 commitment = Message.hash(response);
        _responseCommitments[commitment] = true;

        emit PostResponseEvent(
            response.request.source,
            response.request.dest,
            response.request.from,
            abi.encodePacked(response.request.to),
            response.request.nonce,
            response.request.timeoutTimestamp,
            response.request.body,
            response.request.gaslimit,
            response.response
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
