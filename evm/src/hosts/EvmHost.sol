// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {Context} from "openzeppelin/utils/Context.sol";
import {Math} from "openzeppelin/utils/math/Math.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";

import {IIsmpModule} from "ismp/IIsmpModule.sol";
import {DispatchPost, DispatchPostResponse, DispatchGet} from "ismp/IDispatcher.sol";
import {IIsmpHost, FeeMetadata, ResponseReceipt} from "ismp/IIsmpHost.sol";
import {StateCommitment, StateMachineHeight} from "ismp/IConsensusClient.sol";
import {IHandler} from "ismp/IHandler.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout, Message} from "ismp/Message.sol";

import {IAllowanceTransfer} from "permit2/interfaces/IAllowanceTransfer.sol";
import {ISignatureTransfer} from "permit2/interfaces/ISignatureTransfer.sol";

// The IsmpHost parameters
struct HostParams {
    // default timeout in seconds for requests.
    uint256 defaultTimeout;
    // base fee for GET requests
    uint256 baseGetRequestFee;
    // cost of cross-chain requests in the fee token per byte
    uint256 perByteFee;
    // The fee token contract. This will typically be DAI.
    // but we allow it to be configurable to prevent future regrets.
    address feeTokenAddress;
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
    uint256 lastUpdated;
    // latest state machine height
    uint256 latestStateMachineHeight;
    // Permit2 contract address
    address permit2Address;
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

    // (stateMachineId => (blockHeight => StateCommitment))
    mapping(uint256 => mapping(uint256 => StateCommitment)) private _stateCommitments;

    // (stateMachineId => (blockHeight => timestamp))
    mapping(uint256 => mapping(uint256 => uint256)) private _stateCommitmentsUpdateTime;

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
        uint256 timeoutTimestamp,
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
    function host() public view virtual returns (bytes memory);

    /**
     * @return the mainnet evm chainId for this host
     */
    function chainId() public virtual returns (uint256);

    /**
     * @return the address of the fee token ERC-20 contract on this state machine
     */
    function feeToken() public view returns (address) {
        return _hostParams.feeTokenAddress;
    }

    /**
     * @return the per-byte fee for outgoing requests.
     */
    function perByteFee() external view returns (uint256) {
        return _hostParams.perByteFee;
    }

    /**
     * @return the base fee for outgoing GET requests
     */
    function baseGetRequestFee() external view returns (uint256) {
        return _hostParams.baseGetRequestFee;
    }

    /**
     * @return the state machine identifier for the connected hyperbridge instance
     */
    function hyperbridge() external view returns (bytes memory) {
        return _hostParams.hyperbridge;
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

    function hostParams() external view returns (HostParams memory) {
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
     * @return the challenge period
     */
    function challengePeriod() external view returns (uint256) {
        return _hostParams.challengePeriod;
    }

    /**
     * @return the latest state machine height
     */
    function latestStateMachineHeight() external view returns (uint256) {
        return _hostParams.latestStateMachineHeight;
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
    function setHostParams(HostParams memory params) external onlyManager {
        _hostParams = params;
    }

    /**
     * @dev Updates the HostParams
     * @param params, the new host params. Can only be called by admin on testnets.
     */
    function setHostParamsAdmin(HostParams memory params) external onlyAdmin {
        require(chainId() != block.chainid, "Cannot set params on mainnet");

        _hostParams = params;
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
        _hostParams.lastUpdated = block.timestamp;
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
        _hostParams.latestStateMachineHeight = height;
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
        _hostParams.latestStateMachineHeight = height.height;

        emit StateMachineUpdated({stateMachineId: height.stateMachineId, height: height.height});
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
        // if we're on mainnet, then consensus state can only be initialized once.
        // and updated subsequently by either consensus proofs or cross-chain governance
        if (chainId() == block.chainid) {
            require(_hostParams.consensusState.equals(new bytes(0)), "Unauthorized action");
        }
        _hostParams.latestStateMachineHeight = 0;
        _hostParams.consensusState = state;
    }

    /**
     * @dev Dispatch an incoming post request to destination module
     * @param request - post request
     */
    function dispatchIncoming(PostRequest memory request) external onlyHandler {
        address destination = _bytesToAddress(request.to);
        uint256 size;
        assembly {
            size := extcodesize(destination)
        }
        if (size == 0) {
            // instead of reverting the entire batch, early return here.
            return;
        }

        (bool success,) = address(destination).call(abi.encodeWithSelector(IIsmpModule.onAccept.selector, request));

        if (success) {
            bytes32 commitment = request.hash();
            _requestReceipts[commitment] = tx.origin;

            emit PostRequestHandled({commitment: commitment, relayer: tx.origin});
        }
    }

    /**
     * @dev Dispatch an incoming post response to source module
     * @param response - post response
     */
    function dispatchIncoming(PostResponse memory response) external onlyHandler {
        address origin = _bytesToAddress(response.request.from);
        (bool success,) = address(origin).call(abi.encodeWithSelector(IIsmpModule.onPostResponse.selector, response));

        if (success) {
            bytes32 commitment = response.request.hash();
            _responseReceipts[commitment] = ResponseReceipt({relayer: tx.origin, responseCommitment: response.hash()});

            emit PostResponseHandled({commitment: commitment, relayer: tx.origin});
        }
    }

    /**
     * @dev Dispatch an incoming get response to source module
     * @param response - get response
     */
    function dispatchIncoming(GetResponse memory response, FeeMetadata memory meta) external onlyHandler {
        uint256 fee = 0;
        for (uint256 i = 0; i < response.values.length; i++) {
            fee += (_hostParams.perByteFee * response.values[i].value.length);
        }

        // Charge the originating user/application
        IAllowanceTransfer(_hostParams.permit2Address).transferFrom(
            meta.sender, address(this), uint160(fee), feeToken()
        );

        address origin = _bytesToAddress(response.request.from);
        (bool success,) = address(origin).call(abi.encodeWithSelector(IIsmpModule.onGetResponse.selector, response));

        if (success) {
            bytes32 commitment = response.request.hash();
            // don't commit the full response object because, it's unused.
            _responseReceipts[commitment] = ResponseReceipt({relayer: tx.origin, responseCommitment: bytes32(0)});
            if (meta.fee > 0) {
                require(IERC20(feeToken()).transfer(tx.origin, meta.fee), "EvmHost has insufficient funds");
            }
            emit PostResponseHandled({commitment: commitment, relayer: tx.origin});
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

        IAllowanceTransfer(_hostParams.permit2Address).transferFrom(post.payer, address(this), uint160(fee), feeToken());

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
        uint256 fee = _hostParams.baseGetRequestFee + get.fee;

        IAllowanceTransfer(_hostParams.permit2Address).transferFrom(get.payer, address(this), uint160(fee), feeToken());

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
        _requestCommitments[commitment] = FeeMetadata({sender: get.payer, fee: get.fee});
        emit GetRequestEvent(
            request.source,
            request.dest,
            request.from,
            request.keys,
            request.nonce,
            request.height,
            request.timeoutTimestamp,
            get.fee
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

        IAllowanceTransfer(_hostParams.permit2Address).transferFrom(post.payer, address(this), uint160(fee), feeToken());

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
    function fundRequest(
        bytes32 commitment,
        ISignatureTransfer.PermitTransferFrom memory permit,
        ISignatureTransfer.SignatureTransferDetails calldata transferDetails,
        bytes calldata signature
    ) public {
        FeeMetadata memory metadata = _requestCommitments[commitment];

        require(metadata.sender != address(0), "Unknown request");
        require(metadata.sender != _msgSender(), "User can only fund own requests");
        require(transferDetails.to == address(this), "Invalid approval");

        ISignatureTransfer(_hostParams.permit2Address).permitTransferFrom(
            permit, transferDetails, _msgSender(), signature
        );

        metadata.fee += transferDetails.requestedAmount;
        _requestCommitments[commitment] = metadata;
    }

    /**
     * @dev A fisherman has determined that some [`StateCommitment`]
     *  (which is ideally still in it's challenge period)
     *  is infact fraudulent and misrepresentative of the state
     *  changes at the provided height. This allows them to veto the state commitment.
     *  They aren't required to provide any proofs for this.
     */
    function vetoStateCommitment(StateMachineHeight memory height) public onlyAdmin {
        delete _stateCommitments[height.stateMachineId][height.height];
        delete _stateCommitmentsUpdateTime[height.stateMachineId][height.height];
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
