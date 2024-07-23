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
import {Strings} from "openzeppelin/utils/Strings.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin/token/ERC20/utils/SafeERC20.sol";

import {IIsmpModule, IncomingPostRequest, IncomingPostResponse, IncomingGetResponse} from "ismp/IIsmpModule.sol";
import {DispatchPost, DispatchPostResponse, DispatchGet} from "ismp/IDispatcher.sol";
import {IIsmpHost, FeeMetadata, ResponseReceipt} from "ismp/IIsmpHost.sol";
import {StateCommitment, StateMachineHeight} from "ismp/IConsensusClient.sol";
import {IHandler} from "ismp/IHandler.sol";
import {PostRequest, PostResponse, GetRequest, GetResponse, PostTimeout, Message} from "ismp/Message.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";

// The EvmHost protocol parameters
struct HostParams {
    // The default timeout in seconds for messages. If messages are dispatched
    // with a timeout value lower than this this value will be used instead
    uint256 defaultTimeout;
    // The cost of cross-chain requests in the feeToken per byte,
    // this is charged to the application initiating a request or response
    uint256 perByteFee;
    // The fee token contract address. This will typically be DAI.
    // but we allow it to be configurable to prevent future regrets.
    address feeToken;
    // The admin account, this only has the rights to freeze, or unfreeze the bridge
    address admin;
    // Ismp message handler contract. This performs all verification logic
    // needed to validate cross-chain messages before they are dispatched to local modules
    address handler;
    // The authorized host manager contract, is itself an `IIsmpModule`
    // which receives governance requests from the Hyperbridge chain to either
    // withdraw revenue from the host or update its protocol parameters
    address hostManager;
    // The local UniswapV2Router02 contract, used for swapping the native token to the feeToken.
    address uniswapV2;
    // The unstaking period of Polkadot's validators. In order to prevent long-range attacks
    uint256 unStakingPeriod;
    // Minimum challenge period for state commitments in seconds;
    uint256 challengePeriod;
    // The consensus client contract which handles consensus proof verification
    address consensusClient;
    // State machines whose state commitments are accepted
    uint256[] stateMachines;
    // Privileged set of fishermen accounts
    address[] fishermen;
    // The state machine identifier for hyperbridge
    bytes hyperbridge;
}

/**
 * @title The Host Manager Interface. This provides methods for
 * modifying the host's params or withdrawing bridge revenue.
 *
 * @dev Can only be called used by the HostManager module.
 */
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

// Withdrawal parameters
struct WithdrawParams {
    // The beneficiary address
    address beneficiary;
    // the amount to be disbursed
    uint256 amount;
    // Withdraw the native token?
    bool native;
}

/**
 * @title The EvmHost
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice The IsmpHost and IsmpDispatcher implementation for EVM-compatible chains
 * Refer to the official ISMP specification. https://docs.hyperbridge.network/protocol/ismp
 *
 * @dev The IsmpHost provides the necessary storage interface for the ISMP handlers to process
 * ISMP messages, the IsmpDispatcher provides the interfaces applications use for dispatching requests
 * and responses. This host implementation delegates all verification logic to the IHandler contract.
 * It is only responsible for dispatching incoming & outgoing requests/responses. As well as managing
 * the state of the ISMP protocol.
 */
abstract contract EvmHost is IIsmpHost, IHostManager, Context {
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

    // Monotonically increasing nonce for outgoing requests
    uint256 private _nonce;

    // Emergency shutdown button, only the admin or handler can push this.
    bool private _frozen;

    // Current verified state of the consensus client;
    bytes private _consensusState;

    // Timestamp for when the consensus was most recently updated
    uint256 private _consensusUpdateTimestamp;

    // Emitted when an incoming POST request is handled
    event PostRequestHandled(
        // Commitment of the incoming request
        bytes32 indexed commitment,
        // Relayer responsible for the delivery
        address relayer
    );

    // Emitted when an outgoing POST request timeout is handled, `dest` refers
    // to the destination for the request
    event PostRequestTimeoutHandled(
        // Commitment of the timed out request
        bytes32 indexed commitment,
        // Destination chain for this request
        string dest
    );

    // Emitted when an incoming POST response is handled
    event PostResponseHandled(
        // Commitment of the incoming response
        bytes32 indexed commitment,
        // Relayer responsible for the delivery
        address relayer
    );

    // Emitted when an outgoing POST response timeout is handled, `dest` refers
    // to the destination for the response
    event PostResponseTimeoutHandled(
        // Commitment of the timed out response
        bytes32 indexed commitment,
        // Destination chain for this response
        string dest
    );

    // Emitted when an outgoing GET request is handled
    event GetRequestHandled(
        // Commitment of the GET request
        bytes32 indexed commitment,
        // Relayer responsible for the delivery
        address relayer
    );

    // Emitted when an outgoing GET request timeout is handled, `dest` refers
    // to the destination for the request
    event GetRequestTimeoutHandled(
        // Commitment of the GET request
        bytes32 indexed commitment,
        // Destination chain for this request
        string dest
    );

    // Emitted when new heights are finalized
    event StateMachineUpdated(
        // The state machine that was just updated
        string stateMachineId,
        // The newly updated height
        uint256 height
    );

    // Emitted when a state commitment is vetoed by a fisherman
    event StateCommitmentVetoed(
        // The state machine identifier for the vetoed state commitment
        string stateMachineId,
        // The height that was vetoed
        uint256 height,
        // The state commitment that was vetoed
        StateCommitment stateCommitment,
        // The fisherman responsible for the veto
        address indexed fisherman
    );

    // Emitted when a new POST request is dispatched
    event PostRequestEvent(
        // Source of this request, included for convenience sake
        string source,
        // The destination chain for this request
        string dest,
        // The contract that initiated this request
        address indexed from,
        // The intended recipient module of this request
        bytes to,
        // Monotonically increasing nonce
        uint256 nonce,
        // The timestamp at which this request will be considered as timed out
        uint256 timeoutTimestamp,
        // The serialized request body
        bytes body,
        // The associated relayer fee
        uint256 fee
    );

    // Emitted when a new POST response is dispatched
    event PostResponseEvent(
        // Source of this response, included for convenience sake
        string source,
        // The destination chain for this response
        string dest,
        // The contract that initiated this response
        address indexed from,
        // The intended recipient module of this response
        bytes to,
        // Monotonically increasing nonce
        uint256 nonce,
        // The timestamp at which this request will be considered as timed out
        uint256 timeoutTimestamp,
        // The serialized request body
        bytes body,
        // The serialized response body
        bytes response,
        // The timestamp at which this response will be considered as timed out
        uint256 responseTimeoutTimestamp,
        // The associated relayer fee
        uint256 fee
    );

    // Emitted when a new GET request is dispatched
    event GetRequestEvent(
        // Source of this response, included for convenience sake
        string source,
        // The destination chain for this response
        string dest,
        // The contract that initiated this response
        address indexed from,
        // The requested storage keys
        bytes[] keys,
        // The height for the requested keys
        uint256 height,
        // Monotonically increasing nonce
        uint256 nonce,
        // The timestamp at which this response will be considered as timed out
        uint256 timeoutTimestamp
    );

    // Emitted when a POST or GET request is funded
    event RequestFunded(
        // Commitment of the request
        bytes32 indexed commitment,
        // The updated fee available for relayers
        uint256 newFee
    );

    // Emitted when a POST response is funded
    event PostResponseFunded(
        // Commitment of the response
        bytes32 indexed commitment,
        // The updated fee available for relayers
        uint256 newFee
    );

    // Emitted when the host has either been frozen or unfrozen.
    // A frozen host does not permit any new messages or
    // state commitments.
    event HostFrozen(
        // Frozen status
        bool frozen
    );

    // Emitted when the host params is updated
    event HostParamsUpdated(
        // The old host parameters
        HostParams oldParams,
        // The new host parameters
        HostParams newParams
    );

    // Emitted when the host processes a withdrawal
    event HostWithdrawal(
        // Amount that was withdrawn from the host's feeToken balance
        uint256 amount,
        // The beneficiary address for this withdrawal
        address beneficiary,
        // Flag for if the native token was withdrawn
        bool native
    );

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

    // Failed to withdraw the native token
    error WithdrawalFailed();

    // The IsmpHost has been frozen and cannot dispatch requests
    error FrozenHost();

    // Cannot change the fee token without sweeping all funds from previous one
    error CannotChangeFeeToken();

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

    modifier notFrozen() {
        if (_frozen) revert FrozenHost();
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
     * @return the address of the ERC-20 fee token contract on this state machine
     */
    function feeToken() public view returns (address) {
        return _hostParams.feeToken;
    }

    /**
     * @dev Returns the address for the Uniswap V2 Router implementation used for swaps
     * @return routerAddress - The address to the in-use RouterV02 implementation
     */
    function uniswapV2Router() external view returns (address) {
        return _hostParams.uniswapV2;
    }

    /**
     * @return the per-byte fee for outgoing requests/responses.
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
        if (chainId() == block.chainid) revert UnauthorizedAction();

        uint256 whitelistLength = params.stateMachines.length;
        for (uint256 i = 0; i < whitelistLength; ++i) {
            delete _latestStateMachineHeight[params.stateMachines[i]];
        }
        updateHostParamsInternal(params);
    }

    /**
     * @dev Updates the HostParams. Will reset all fishermen accounts and initialize any new state machines.
     * @param params, the new host params.
     */
    function updateHostParamsInternal(HostParams memory params) internal {
        // check if the provided host manager is a contract
        if (params.hostManager == address(0) || address(params.hostManager).code.length == 0) {
            revert InvalidHostManagerAddress();
        }

        // we can only have a maximum of 100 fishermen
        uint256 newFishermenLength = params.fishermen.length;
        if (newFishermenLength > 100) revert MaxFishermanCountExceeded(newFishermenLength);

        // reset old fishermen
        uint256 fishermenLength = _hostParams.fishermen.length;
        for (uint256 i = 0; i < fishermenLength; ++i) {
            delete _fishermen[_hostParams.fishermen[i]];
        }

        if (_hostParams.feeToken != address(0) && _hostParams.feeToken != params.feeToken) {
            uint256 balance = IERC20(_hostParams.feeToken).balanceOf(address(this));
            if (balance != 0) revert CannotChangeFeeToken();
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
        uint256 whitelistLength = params.stateMachines.length;
        for (uint256 i = 0; i < whitelistLength; ++i) {
            // create if it doesn't already exist
            if (_latestStateMachineHeight[params.stateMachines[i]] == 0) {
                _latestStateMachineHeight[params.stateMachines[i]] = 1;
            }
        }
    }

    /**
     * @dev withdraws host revenue to the given address, can only be called by cross-chain governance
     * @param params, the parameters for withdrawal
     */
    function withdraw(WithdrawParams memory params) external restrict(_hostParams.hostManager) {
        if (params.native) {
            (bool sent, ) = params.beneficiary.call{value: params.amount}("");
            if (!sent) revert WithdrawalFailed();
        } else {
            SafeERC20.safeTransfer(IERC20(feeToken()), params.beneficiary, params.amount);
        }
        emit HostWithdrawal({beneficiary: params.beneficiary, amount: params.amount, native: params.native});
    }

    /**
     * @dev Store the serialized consensus state, alongside relevant metadata
     */
    function storeConsensusState(bytes memory state) external restrict(_hostParams.handler) {
        _consensusState = state;
        _consensusUpdateTimestamp = block.timestamp;
    }

    /**
     * @dev Store the state commitment at given state height alongside relevant metadata.
     * Assumes the state commitment is of the latest height.
     */
    function storeStateMachineCommitment(
        StateMachineHeight memory height,
        StateCommitment memory commitment
    ) external restrict(_hostParams.handler) {
        _stateCommitments[height.stateMachineId][height.height] = commitment;
        _stateCommitmentsUpdateTime[height.stateMachineId][height.height] = block.timestamp;
        _latestStateMachineHeight[height.stateMachineId] = height.height;

        emit StateMachineUpdated({
            stateMachineId: this.stateMachineId(_hostParams.hyperbridge, height.stateMachineId),
            height: height.height
        });
    }

    /**
     * @dev Delete the state commitment at given state height.
     */
    function deleteStateMachineCommitment(
        StateMachineHeight memory height,
        address fisherman
    ) external restrict(_hostParams.handler) {
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
    function deleteStateMachineCommitmentInternal(StateMachineHeight memory height, address fisherman) internal {
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
            stateMachineId: this.stateMachineId(_hostParams.hyperbridge, height.stateMachineId),
            stateCommitment: stateCommitment,
            height: height.height,
            fisherman: fisherman
        });
    }

    /**
     * @dev Get the state machine id for a parachain
     */
    function stateMachineId(bytes calldata parachainId, uint256 id) external pure returns (string memory) {
        uint256 offset = parachainId.length - 4;
        return string.concat(string(parachainId[:offset]), Strings.toString(id));
    }

    /**
     * @dev set the new state of the bridge
     * @param newState new state
     */
    function setFrozenState(bool newState) external {
        if (_msgSender() != _hostParams.admin && _msgSender() != _hostParams.handler) revert UnauthorizedAction();

        _frozen = newState;

        emit HostFrozen({frozen: newState});
    }

    /**
     * @dev sets the initial consensus state
     * @param state initial consensus state
     */
    function setConsensusState(
        bytes memory state,
        StateMachineHeight memory height,
        StateCommitment memory commitment
    ) public restrict(_hostParams.admin) {
        // if we're on mainnet, then consensus state can only be initialized once
        // and updated subsequently through consensus proofs
        bool canUpdate = chainId() == block.chainid ? keccak256(_consensusState) == keccak256(bytes("")) : true;
        if (!canUpdate) revert UnauthorizedAction();

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

        (bool success, ) = address(destination).call(
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
        _responseReceipts[requestCommitment] = ResponseReceipt({
            relayer: relayer,
            responseCommitment: responseCommitment
        });

        (bool success, ) = address(origin).call(
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

        (bool success, ) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onGetResponse.selector, IncomingGetResponse(response, relayer))
        );

        if (!success) {
            // so that it can be retried
            delete _responseReceipts[commitment];
            return;
        }

        emit GetRequestHandled({commitment: commitment, relayer: relayer});
    }

    /**
     * @dev Dispatch an incoming GET timeout to the source module.
     * @notice Does not refund any protocol fees.
     * @param request - get request
     */
    function dispatchTimeOut(
        GetRequest memory request,
        FeeMetadata memory meta,
        bytes32 commitment
    ) external restrict(_hostParams.handler) {
        address origin = _bytesToAddress(request.from);

        // replay protection
        delete _requestCommitments[commitment];
        (bool success, ) = address(origin).call(abi.encodeWithSelector(IIsmpModule.onGetTimeout.selector, request));

        if (!success) {
            // so that it can be retried
            _requestCommitments[commitment] = meta;
            return;
        }

        emit GetRequestTimeoutHandled({commitment: commitment, dest: string(request.dest)});
    }

    /**
     * @dev Dispatch an incoming POST timeout to the source module
     * @param request - post timeout
     */
    function dispatchTimeOut(
        PostRequest memory request,
        FeeMetadata memory meta,
        bytes32 commitment
    ) external restrict(_hostParams.handler) {
        address origin = _bytesToAddress(request.from);

        // replay protection
        delete _requestCommitments[commitment];
        (bool success, ) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onPostRequestTimeout.selector, request)
        );

        if (!success) {
            // so that it can be retried
            _requestCommitments[commitment] = meta;
            return;
        }

        if (meta.fee != 0) {
            SafeERC20.safeTransfer(IERC20(feeToken()), meta.sender, meta.fee);
        }
        emit PostRequestTimeoutHandled({commitment: commitment, dest: string(request.dest)});
    }

    /**
     * @dev Dispatch an incoming POST response timeout to the source module
     * @param response - timed-out post response
     */
    function dispatchTimeOut(
        PostResponse memory response,
        FeeMetadata memory meta,
        bytes32 commitment
    ) external restrict(_hostParams.handler) {
        address origin = _bytesToAddress(response.request.to);

        // replay protection
        bytes32 reqCommitment = response.request.hash();
        delete _responseCommitments[commitment];
        delete _responded[reqCommitment];
        (bool success, ) = address(origin).call(
            abi.encodeWithSelector(IIsmpModule.onPostResponseTimeout.selector, response)
        );

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
        emit PostResponseTimeoutHandled({commitment: commitment, dest: string(response.request.source)});
    }

    /**
	 * @dev Dispatch a POST request to Hyperbridge
	 *
	 * @notice Payment for the request can be made with either the native token or the IIsmpHost.feeToken.
	 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
	 * Will revert if enough native tokens are not provided.
	 *
	 * If no native tokens are provided then it will try to collect payment from the calling contract in
	 * the IIsmpHost.feeToken.
	 *
	 * @param post - post request
	 * @return commitment - the request commitment
	 */
    function dispatch(DispatchPost memory post) external payable notFrozen returns (bytes32 commitment) {
        uint256 fee = (_hostParams.perByteFee * post.body.length) + post.fee;

        if (msg.value > 0) {
            address[] memory path = new address[](2);
            path[0] = IUniswapV2Router02(_hostParams.uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(_hostParams.uniswapV2).swapETHForExactTokens{value: msg.value}(
                fee,
                path,
                address(this),
                block.timestamp
            );
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), fee);
        }
        // adjust the timeout
        uint256 timeout = _hostParams.defaultTimeout > post.timeout ? _hostParams.defaultTimeout : post.timeout;
        uint64 timeoutTimestamp = post.timeout == 0 ? 0 : uint64(block.timestamp) + uint64(timeout);
        PostRequest memory request = PostRequest({
            source: host(),
            dest: post.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            to: post.to,
            timeoutTimestamp: timeoutTimestamp,
            body: post.body
        });

        // make the commitment
        commitment = request.hash();
        _requestCommitments[commitment] = FeeMetadata({sender: post.payer, fee: post.fee});
        emit PostRequestEvent({
            source: string(request.source),
            dest: string(request.dest),
            from: _msgSender(),
            to: abi.encodePacked(request.to),
            nonce: request.nonce,
            timeoutTimestamp: request.timeoutTimestamp,
            body: request.body,
            fee: post.fee
        });
    }

	/**
	 * @dev Dispatch a GET request to Hyperbridge
	 *
	 * @notice Payment for the request can be made with either the native token or the IIsmpHost.feeToken.
	 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
	 * Will revert if enough native tokens are not provided.
	 *
	 * If no native tokens are provided then it will try to collect payment from the calling contract in
	 * the IIsmpHost.feeToken.
	 *
	 * @param get - get request
	 * @return commitment - the request commitment
	 */
    function dispatch(DispatchGet memory get) external payable notFrozen returns (bytes32 commitment) {
        if (get.fee != 0) {
            if (msg.value > 0) {
                address[] memory path = new address[](2);
                path[0] = IUniswapV2Router02(_hostParams.uniswapV2).WETH();
                path[1] = feeToken();
                IUniswapV2Router02(_hostParams.uniswapV2).swapETHForExactTokens{value: msg.value}(
                    get.fee,
                    path,
                    address(this),
                    block.timestamp
                );
            } else {
                SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), get.fee);
            }
        }
        // adjust the timeout
        uint256 timeout = _hostParams.defaultTimeout > get.timeout ? _hostParams.defaultTimeout : get.timeout;
        uint64 timeoutTimestamp = get.timeout == 0 ? 0 : uint64(block.timestamp) + uint64(timeout);

        GetRequest memory request = GetRequest({
            source: host(),
            dest: get.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            timeoutTimestamp: timeoutTimestamp,
            keys: get.keys,
            height: get.height
        });

        // make the commitment
        commitment = request.hash();
        _requestCommitments[commitment] = FeeMetadata({sender: msg.sender, fee: get.fee});
        emit GetRequestEvent({
            source: string(request.source),
            dest: string(request.dest),
            from: _msgSender(),
            keys: request.keys,
            nonce: request.nonce,
            height: request.height,
            timeoutTimestamp: request.timeoutTimestamp
        });
    }

	/**
	 * @dev Dispatch a POST response to Hyperbridge
	 *
	 * @notice Payment for the request can be made with either the native token or the IIsmpHost.feeToken.
	 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
	 * Will revert if enough native tokens are not provided.
	 *
	 * If no native tokens are provided then it will try to collect payment from the calling contract in
	 * the IIsmpHost.feeToken.
	 *
	 * @param post - post response
	 * @return commitment - the request commitment
	 */
    function dispatch(DispatchPostResponse memory post) external payable notFrozen returns (bytes32 commitment) {
        uint256 fee = (_hostParams.perByteFee * post.response.length) + post.fee;
        if (msg.value > 0) {
            address[] memory path = new address[](2);
            path[0] = IUniswapV2Router02(_hostParams.uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(_hostParams.uniswapV2).swapETHForExactTokens{value: msg.value}(
                fee,
                path,
                address(this),
                block.timestamp
            );
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), fee);
        }

        bytes32 receipt = post.request.hash();
        address caller = _msgSender();

        // known request?
        if (_requestReceipts[receipt] == address(0)) revert UnknownRequest();

        // check that the authorized application is issuing this response
        if (_bytesToAddress(post.request.to) != caller) revert UnauthorizedResponse();

        // check that request has not already been respond to
        if (_responded[receipt]) revert DuplicateResponse();

        // adjust the timeout
        uint256 timeout = _hostParams.defaultTimeout > post.timeout ? _hostParams.defaultTimeout : post.timeout;
        uint64 timeoutTimestamp = post.timeout == 0 ? 0 : uint64(block.timestamp) + uint64(timeout);

        PostResponse memory response = PostResponse({
            request: post.request,
            response: post.response,
            timeoutTimestamp: timeoutTimestamp
        });
        commitment = response.hash();

        FeeMetadata memory meta = FeeMetadata({fee: post.fee, sender: post.payer});
        _responseCommitments[commitment] = meta;
        _responded[receipt] = true;

        // note the swapped fields
        emit PostResponseEvent({
            source: string(response.request.dest),
            dest: string(response.request.source),
            from: caller,
            to: response.request.from,
            nonce: response.request.nonce,
            timeoutTimestamp: response.request.timeoutTimestamp,
            body: response.request.body,
            response: response.response,
            responseTimeoutTimestamp: response.timeoutTimestamp,
            fee: meta.fee // sigh solidity
        });
    }

    /**
     * @dev Increase the relayer fee for a previously dispatched request.
     * This is provided for use only on pending requests, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * @notice Payment can be made with either the native token or the IIsmpHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IIsmpHost.feeToken.
     *
     * If called on an already delivered request, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The request commitment
     * @param amount - The amount provided in `IIsmpHost.feeToken()`
     */
    function fundRequest(bytes32 commitment, uint256 amount) external payable {
        if (msg.value > 0) {
            address[] memory path = new address[](2);
            path[0] = IUniswapV2Router02(_hostParams.uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(_hostParams.uniswapV2).swapETHForExactTokens{value: msg.value}(
                amount,
                path,
                address(this),
                block.timestamp
            );
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), amount);
        }
        FeeMetadata memory metadata = _requestCommitments[commitment];

        if (metadata.sender == address(0)) revert UnknownRequest();

        metadata.fee += amount;
        _requestCommitments[commitment] = metadata;

        emit RequestFunded({commitment: commitment, newFee: metadata.fee});
    }

    /**
     * @dev Increase the relayer fee for a previously dispatched response.
     * This is provided for use only on pending responses, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * @notice Payment can be made with either the native token or the IIsmpHost.feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the IIsmpHost.feeToken.
     *
     * If called on an already delivered response, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The response commitment
     * @param amount - The amount to be provided in `IIsmpHost.feeToken()`
     */
    function fundResponse(bytes32 commitment, uint256 amount) external payable {
        if (msg.value > 0) {
            address[] memory path = new address[](2);
            path[0] = IUniswapV2Router02(_hostParams.uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(_hostParams.uniswapV2).swapETHForExactTokens{value: msg.value}(
                amount,
                path,
                address(this),
                block.timestamp
            );
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken()), _msgSender(), address(this), amount);
        }

        FeeMetadata memory metadata = _responseCommitments[commitment];

        if (metadata.sender == address(0)) revert UnknownResponse();

        metadata.fee += amount;
        _responseCommitments[commitment] = metadata;

        emit PostResponseFunded({commitment: commitment, newFee: metadata.fee});
    }

    /*
     * @dev receive function for UniswapV2Router02, collects all dust native tokens.
     */
    receive() external payable {}

    /**
     * @dev Get next available nonce for outgoing requests.
     */
    function _nextNonce() internal returns (uint256) {
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
    function _bytesToAddress(bytes memory _bytes) internal pure returns (address addr) {
        if (_bytes.length != 20) revert InvalidAddressLength();

        assembly {
            addr := mload(add(_bytes, 20))
        }
    }
}
