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

import {Context} from "@openzeppelin/contracts/utils/Context.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC165} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";

import {
    IApp,
    IncomingPostRequest,
    IncomingGetResponse,
    PostRequestTimeout,
    GetRequestTimeout
} from "@hyperbridge/core/interfaces/IApp.sol";
import {DispatchPost, DispatchGet} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IHost, FeeMetadata, ResponseReceipt, FrozenStatus} from "@hyperbridge/core/interfaces/IHost.sol";
import {StateCommitment, StateMachineHeight} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {IHandler} from "@hyperbridge/core/interfaces/IHandler.sol";
import {PostRequest, GetRequest, GetResponse, Message} from "@hyperbridge/core/libraries/Message.sol";
import {IConsensus} from "@hyperbridge/core/interfaces/IConsensus.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";

// The EvmHost protocol parameters
struct HostParams {
    // The fee token contract address. This will typically be DAI.
    // but we allow it to be configurable to prevent future regrets.
    address feeToken;
    // The admin account, this only has the rights to freeze, or unfreeze the bridge
    address admin;
    // Ismp message handler contract. This performs all verification logic
    // needed to validate cross-chain messages before they are dispatched to local modules
    address handler;
    // The authorized host manager contract, is itself an `IApp`
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
    address token;
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
contract EvmHost is IHost, IHostManager, Context {
    using Message for PostRequest;
    using Message for GetRequest;
    using Message for GetResponse;
    using SafeERC20 for IERC20;

    // commitment of all outgoing requests and amount put up for relayers.
    mapping(bytes32 => FeeMetadata) private _requestCommitments;

    // commitment of all incoming requests and who delivered them.
    mapping(bytes32 => address) private _requestReceipts;

    // commitment of all incoming responses and who delivered them.
    // maps the request commitment to a receipt object
    mapping(bytes32 => ResponseReceipt) private _responseReceipts;

    // mapping of state machine identifier to latest known height to state commitment
    // (stateMachineId => (blockHeight => StateCommitment))
    mapping(uint256 => mapping(uint256 => StateCommitment)) private _stateCommitments;

    // mapping of state machine identifier to latest known height to update time
    // (stateMachineId => (blockHeight => timestamp))
    mapping(uint256 => mapping(uint256 => uint256)) private _stateCommitmentsUpdateTime;

    // mapping of state machine identifier to latest known height
    // (stateMachineId => blockHeight)
    mapping(uint256 => uint256) internal _latestStateMachineHeight;

    // mapping of state machine identifier to height vetoed to fisherman
    // useful for rewarding fishermen on hyperbridge
    // (stateMachineId => (blockHeight => fisherman))
    mapping(uint256 => mapping(uint256 => address)) private _vetoes;

    // Parameters for the host
    HostParams internal _hostParams;

    // Monotonically increasing nonce for outgoing requests
    uint256 private _nonce;

    // Frozen status of the host, only the admin or handler can change this.
    FrozenStatus private _frozen;

    // Current verified state of the consensus client;
    bytes private _consensusState;

    // Timestamp for when the consensus was most recently updated
    uint256 private _consensusUpdateTimestamp;

    // Maps an authority set ID (epoch) to the relayer that first submitted
    // the consensus proof for that epoch.
    mapping(uint256 => address) private _epochs;

    // The most recent authority set ID for which a consensus proof has been submitted.
    uint256 private _currentEpoch;

    // One-shot guard for `initialize`. Appended after all existing storage so
    // this change does not shift any pre-existing storage slots.
    bool private _initialized;

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

    // Emitted when a new GET request is dispatched
    event GetRequestEvent(
        // Source of this response, included for convenience sake
        string source,
        // The destination chain for this response
        string dest,
        // The contract that initiated this response
        bytes from,
        // The requested storage keys
        bytes[] keys,
        // The height for the requested keys
        uint256 height,
        // Monotonically increasing nonce
        uint256 nonce,
        // The timestamp at which this response will be considered as timed out
        uint256 timeoutTimestamp,
        // Some application-specific metadata relating to this request
        bytes context,
        // The associated protocol fee
        uint256 fee
    );

    // Emitted when a POST or GET request is funded
    event RequestFunded(
        // Commitment of the request
        bytes32 indexed commitment,
        // The updated fee available for relayers
        uint256 newFee
    );

    // Emitted when the frozen status of the host changes
    event HostFrozen(
        // Frozen status
        FrozenStatus status
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
        // The token that was withdrawn
        address token
    );

    // Emitted when a consensus proof introduces a new authority set epoch
    event NewEpoch(
        // The new authority set ID
        uint256 indexed authoritySetId,
        // The address of the relayer that submitted the proof
        address indexed relayer
    );

    // Account is unauthorized to perform requested action
    error UnauthorizedAccount();

    // Provided address didn't fit address type size
    error InvalidAddressLength();

    // Provided request was unknown
    error UnknownRequest();

    // Action breaks protocol invariants and is therefore unauthorized
    error UnauthorizedAction();

    // Host manager address was zero, not a contract or didn't meet it's required ERC165 interface.
    error InvalidHostManager();

    // Handler address was zero, not a contract or didn't meet it's required ERC165 interface.
    error InvalidHandler();

    // Consensus client address was zero, not a contract or didn't meet it's required ERC165 interface.
    error InvalidConsensusClient();

    // Provided an empty Hyperbridge stateMachineId during host params update
    error InvalidHyperbridgeId();

    // Provided an empty array of stateMachines during host params update
    error InvalidStateMachinesLength();

    // Provided an unstaking period less than 24 hours
    error InvalidUnstakingPeriod();

    // Failed to withdraw the native token
    error WithdrawalFailed();

    // The IsmpHost has been frozen and cannot dispatch requests
    error FrozenHost();

    // Cannot change the fee token without sweeping all funds from previous one
    error CannotChangeFeeToken();

    // restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (_msgSender() != caller) revert UnauthorizedAction();
        _;
    }

    /*
     * @dev Check if outgoing messages are permitted
     */
    modifier notFrozen() {
        if (_frozen == FrozenStatus.Outgoing || _frozen == FrozenStatus.All) revert FrozenHost();
        _;
    }

    /**
     * @dev Constructor only sets the initial admin and the consensus update
     * timestamp. All other configuration is deferred to `initialize` so that
     * the constructor's init code is identical on every chain (assuming the
     * same `admin` is used everywhere), which is required for CREATE2 address
     * parity across chains.
     */
    constructor(address _admin) {
        _consensusUpdateTimestamp = block.timestamp;
        _hostParams.admin = _admin;
    }

    /**
     * @dev One-shot initializer. Can only be called once, and only by the
     * admin set in the constructor. Applies the initial `HostParams` via
     * the internal updater.
     */
    function initialize(HostParams memory params) external {
        if (_msgSender() != _hostParams.admin) revert UnauthorizedAction();
        if (_initialized) revert UnauthorizedAction();
        _initialized = true;
        updateHostParamsInternal(params);
    }

    /*
     * @dev receive function for UniswapV2Router02, collects all dust native tokens.
     */
    receive() external payable {}

    /**
     * @return the host state machine id
     */
    function host() public view returns (bytes memory) {
        return StateMachine.evm(block.chainid);
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
    function frozen() external view returns (FrozenStatus) {
        return _frozen;
    }

    /**
     * @dev Returns the nonce immediately available for requests
     * @return the `nonce`
     */
    function nonce() external view returns (uint256) {
        return _nonce;
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
     * @return the most recent authority set ID (epoch) for which a consensus proof has been submitted
     */
    function currentEpoch() external view returns (uint256) {
        return _currentEpoch;
    }

    /**
     * @dev Returns the relayer that first submitted the consensus proof for the given epoch.
     * @param authoritySetId - the authority set / epoch ID
     * @return the relayer address, or address(0) if not set
     */
    function relayerOf(uint256 authoritySetId) external view returns (address) {
        return _epochs[authoritySetId];
    }

    /**
     * @return the `HostParams`
     */
    function hostParams() external view returns (HostParams memory) {
        return _hostParams;
    }

    /**
     * @return the host admin
     */
    function admin() external view returns (address) {
        return _hostParams.admin;
    }

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
     * @return the state machine identifier for the connected hyperbridge instance
     */
    function hyperbridge() external view returns (bytes memory) {
        return _hostParams.hyperbridge;
    }

    /**
     * @dev Should return a handle to the consensus client based on the id
     * @return the consensus client contract
     */
    function consensusClient() external view returns (address) {
        return _hostParams.consensusClient;
    }

    /**
     * @return the challenge period
     */
    function challengePeriod() external view returns (uint256) {
        return _hostParams.challengePeriod;
    }

    /**
     * @return the unstaking period
     */
    function unStakingPeriod() external view returns (uint256) {
        return _hostParams.unStakingPeriod;
    }

    /**
     * @return the latest state machine height for the given stateMachineId. If it returns 0, the state machine is unsupported.
     */
    function latestStateMachineHeight(uint256 id) external view returns (uint256) {
        return _latestStateMachineHeight[id];
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
     * @dev Returns the fisherman responsible for vetoing the given state machine height.
     * @return the `fisherman` address
     */
    function vetoes(uint256 paraId, uint256 height) external view returns (address) {
        return _vetoes[paraId][height];
    }

    /**
     * @param height - state machine height
     * @return the state machine update time at `height`
     */
    function stateMachineCommitmentUpdateTime(StateMachineHeight memory height) external view returns (uint256) {
        return _stateCommitmentsUpdateTime[height.stateMachineId][height.height];
    }

    /**
     * @param height - state machine height
     * @return the state commitment at `height`
     */
    function stateMachineCommitment(StateMachineHeight memory height)
        external
        payable
        returns (StateCommitment memory)
    {
        return _stateCommitments[height.stateMachineId][height.height];
    }

    /**
     * @dev Updates the HostParams. Only callable by cross-chain governance
     * via the configured `hostManager`. The admin has no privileges here —
     * environments that need a privileged admin override (testnets, forks)
     * should use `TestnetHost`, which extends this contract.
     *
     * Marked `virtual` so subclasses can broaden the authorization
     * @param params, the new host params.
     */
    function updateHostParams(HostParams memory params) external virtual restrict(_hostParams.hostManager) {
        updateHostParamsInternal(params);
    }

    /**
     * @dev Updates the HostParams. Will reset all fishermen accounts and initialize any new state machines.
     * @param params, the new host params.
     */
    function updateHostParamsInternal(HostParams memory params) internal {
        // check the params to prevent the host from getting bricked.
        if (
            params.hostManager == address(0) || address(params.hostManager).code.length == 0
                || !IERC165(params.hostManager).supportsInterface(type(IApp).interfaceId)
        ) {
            // otherwise cannot process new cross-chain governance requests
            revert InvalidHostManager();
        }

        if (
            params.handler == address(0) || address(params.handler).code.length == 0
                || !IERC165(params.handler).supportsInterface(type(IHandler).interfaceId)
        ) {
            // otherwise cannot process new datagrams
            revert InvalidHandler();
        }

        if (
            params.consensusClient == address(0) || address(params.consensusClient).code.length == 0
                || !IERC165(params.consensusClient).supportsInterface(type(IConsensus).interfaceId)
        ) {
            // otherwise cannot process new consensus datagrams
            revert InvalidConsensusClient();
        }

        // otherwise cannot process new cross-chain governance requests
        if (keccak256(params.hyperbridge) == keccak256(bytes(""))) revert InvalidHyperbridgeId();

        // otherwise cannot process new datagrams
        uint256 stateMachinesLen = params.stateMachines.length;
        if (stateMachinesLen == 0) revert InvalidStateMachinesLength();

        // otherwise cannot process new datagrams
        if (1 days > params.unStakingPeriod) revert InvalidUnstakingPeriod();

        address oldFeeToken = feeToken();
        if (oldFeeToken != address(0) && oldFeeToken != params.feeToken) {
            uint256 balance = IERC20(oldFeeToken).balanceOf(address(this));
            if (balance != 0) revert CannotChangeFeeToken();
        }

        // safe to emit here because invariants have already been checked
        // and don't want to store a temp variable for the old params
        emit HostParamsUpdated({oldParams: _hostParams, newParams: params});

        _hostParams.feeToken = params.feeToken;
        _hostParams.admin = params.admin;
        _hostParams.handler = params.handler;
        _hostParams.hostManager = params.hostManager;
        _hostParams.uniswapV2 = params.uniswapV2;
        _hostParams.unStakingPeriod = params.unStakingPeriod;
        _hostParams.challengePeriod = params.challengePeriod;
        _hostParams.consensusClient = params.consensusClient;
        _hostParams.stateMachines = params.stateMachines;
        _hostParams.hyperbridge = params.hyperbridge;

        // add whitelisted state machines
        for (uint256 i = 0; i < stateMachinesLen; ++i) {
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
        if (params.token == address(0)) {
            // this is safe because re-entrancy is mitigated before dispatching requests
            (bool sent,) = params.beneficiary.call{value: params.amount}("");
            if (!sent) revert WithdrawalFailed();
        } else {
            IERC20(params.token).safeTransfer(params.beneficiary, params.amount);
        }
        emit HostWithdrawal({beneficiary: params.beneficiary, amount: params.amount, token: params.token});
    }

    /**
     * @dev Store the serialized consensus state, alongside relevant metadata
     */
    function storeConsensusState(bytes memory state) external restrict(_hostParams.handler) {
        _consensusState = state;
        _consensusUpdateTimestamp = block.timestamp;
    }

    /**
     * @dev Record the relayer that first submitted a consensus proof for a new authority set epoch.
     * Only callable by the configured handler. Stale or duplicate epoch IDs are ignored.
     * @param authoritySetId the new authority set / epoch ID
     * @param relayer the relayer that delivered the consensus proof
     */
    function recordEpoch(uint256 authoritySetId, address relayer) external restrict(_hostParams.handler) {
        if (authoritySetId <= _currentEpoch) return;
        _currentEpoch = authoritySetId;
        _epochs[authoritySetId] = relayer;
        emit NewEpoch({authoritySetId: authoritySetId, relayer: relayer});
    }

    /**
     * @dev Store the state commitment at given state height alongside relevant metadata.
     * Assumes the state commitment is of the latest height.
     */
    function storeStateMachineCommitment(StateMachineHeight memory height, StateCommitment memory commitment)
        external
        restrict(_hostParams.handler)
    {
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
    function deleteStateMachineCommitment(StateMachineHeight memory height, address fisherman)
        external
        restrict(_hostParams.handler)
    {
        deleteStateMachineCommitmentInternal(height, fisherman);
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
    function setFrozenState(FrozenStatus newState) external {
        address caller = _msgSender();
        if (caller != _hostParams.admin && caller != _hostParams.handler) revert UnauthorizedAction();

        _frozen = newState;

        emit HostFrozen({status: newState});
    }

    /**
     * @dev Whether the admin is permitted to (re)initialize the consensus
     * state. On mainnet hosts the consensus state may only be set once via
     * `setConsensusState` and is thereafter only updated through consensus
     * proofs. `TestnetHost` overrides this to permit repeated admin
     * re-initialization.
     */
    function _canReinitConsensus() internal view virtual returns (bool) {
        return keccak256(_consensusState) == keccak256(bytes(""));
    }

    /**
     * @dev sets the initial consensus state. By default this is a one-shot
     * operation: once `_consensusState` is non-empty the admin can no longer
     * call this and consensus state moves only through `storeConsensusState`
     * (handler-only, driven by consensus proofs). `TestnetHost` overrides
     * `_canReinitConsensus` to lift this restriction.
     * @param state initial consensus state
     * @param height initial state-machine height
     * @param commitment initial state commitment at `height`
     */
    function setConsensusState(bytes memory state, StateMachineHeight memory height, StateCommitment memory commitment)
        public
        restrict(_hostParams.admin)
    {
        if (!_canReinitConsensus()) revert UnauthorizedAction();

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

        (bool success,) = address(destination)
            .call(abi.encodeWithSelector(IApp.onAccept.selector, IncomingPostRequest(request, relayer)));

        if (!success) {
            // so that it can be retried
            delete _requestReceipts[commitment];
            return;
        }
        emit PostRequestHandled({commitment: commitment, relayer: relayer});
    }

    /**
     * @dev Dispatch an incoming GET response to source module
     * @param response - get response
     */
    function dispatchIncoming(GetResponse memory response, address relayer) external restrict(_hostParams.handler) {
        // replay protection
        bytes32 commitment = response.request.hash();
        _responseReceipts[commitment] = ResponseReceipt({
            relayer: relayer,
            responseCommitment: response.hash()
        });

        (bool success,) = _bytesToAddress(response.request.from)
            .call(abi.encodeWithSelector(IApp.onGetResponse.selector, IncomingGetResponse(response, relayer)));

        if (!success) {
            // so that it can be retried
            delete _responseReceipts[commitment];
            return;
        }

        // reward the relayer fee
        uint256 fee = _requestCommitments[commitment].fee;
        if (fee != 0) {
            IERC20(feeToken()).safeTransfer(relayer, fee);
        }
        emit GetRequestHandled({commitment: commitment, relayer: relayer});
    }

    /**
     * @dev Dispatch an incoming GET timeout to the source module.
     * @notice Does not refund any protocol fees.
     * @param timeout - timed-out get request bundled with the relayer that submitted the timeout proof
     * @param meta - fee metadata for the original request
     * @param commitment - request commitment
     */
    function dispatchTimeOut(
        GetRequestTimeout memory timeout,
        FeeMetadata memory meta,
        bytes32 commitment
    ) external restrict(_hostParams.handler) {
        // replay protection
        delete _requestCommitments[commitment];
        (bool success,) = _bytesToAddress(timeout.request.from)
            .call(abi.encodeWithSelector(IApp.onGetTimeout.selector, timeout));

        if (!success) {
            // so that it can be retried
            _requestCommitments[commitment] = meta;
            return;
        }

        if (meta.fee != 0) {
            // refund relayer fee
            IERC20(feeToken()).safeTransfer(meta.sender, meta.fee);
        }
        emit GetRequestTimeoutHandled({commitment: commitment, dest: string(timeout.request.dest)});
    }

    /**
     * @dev Dispatch an incoming POST timeout to the source module
     * @param timeout - timed-out post request bundled with the relayer that submitted the timeout proof
     * @param meta - fee metadata for the original request
     * @param commitment - request commitment
     */
    function dispatchTimeOut(
        PostRequestTimeout memory timeout,
        FeeMetadata memory meta,
        bytes32 commitment
    ) external restrict(_hostParams.handler) {
        // replay protection
        delete _requestCommitments[commitment];
        (bool success,) = _bytesToAddress(timeout.request.from)
            .call(abi.encodeWithSelector(IApp.onPostRequestTimeout.selector, timeout));

        if (!success) {
            // so that it can be retried
            _requestCommitments[commitment] = meta;
            return;
        }

        if (meta.fee != 0) {
            // refund relayer fee
            IERC20(feeToken()).safeTransfer(meta.sender, meta.fee);
        }
        emit PostRequestTimeoutHandled({commitment: commitment, dest: string(timeout.request.dest)});
    }

    /**
     * @dev Dispatch a POST request to Hyperbridge
     *
     * @notice Payment for the request can be made with either the native token or the feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the feeToken.
     *
     * @param post - post request
     * @return commitment - the request commitment
     */
    function dispatch(DispatchPost memory post) external payable notFrozen returns (bytes32 commitment) {
        if (msg.value > 0) {
            address[] memory path = new address[](2);
            address uniswapV2 = _hostParams.uniswapV2;
            path[0] = IUniswapV2Router02(uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msg.value}(
                post.fee, path, address(this), block.timestamp
            );
        } else if (post.fee > 0) {
            IERC20(feeToken()).safeTransferFrom(_msgSender(), address(this), post.fee);
        }

        // adjust the timeout
        uint64 timeoutTimestamp = post.timeout == 0 ? 0 : uint64(block.timestamp) + uint64(post.timeout);
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
     * @notice Payment for the request can be made with either the native token or the feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the feeToken.
     *
     * @param get - get request
     * @return commitment - the request commitment
     */
    function dispatch(DispatchGet memory get) external payable notFrozen returns (bytes32 commitment) {
        if (msg.value > 0) {
            address[] memory path = new address[](2);
            address uniswapV2 = _hostParams.uniswapV2;
            path[0] = IUniswapV2Router02(uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msg.value}(
                get.fee, path, address(this), block.timestamp
            );
        } else if (get.fee > 0) {
            IERC20(feeToken()).safeTransferFrom(_msgSender(), address(this), get.fee);
        }

        uint64 timeoutTimestamp = get.timeout == 0 ? 0 : uint64(block.timestamp) + uint64(get.timeout);
        GetRequest memory request = GetRequest({
            source: host(),
            dest: get.dest,
            nonce: uint64(_nextNonce()),
            from: abi.encodePacked(_msgSender()),
            timeoutTimestamp: timeoutTimestamp,
            keys: get.keys,
            height: get.height,
            context: get.context
        });

        // make the commitment
        commitment = request.hash();
        _requestCommitments[commitment] = FeeMetadata({sender: _msgSender(), fee: get.fee});
        emit GetRequestEvent({
            source: string(request.source),
            dest: string(request.dest),
            from: request.from,
            keys: request.keys,
            nonce: request.nonce,
            height: request.height,
            context: request.context,
            timeoutTimestamp: request.timeoutTimestamp,
            fee: get.fee
        });
    }

    /**
     * @dev Increase the relayer fee for a previously dispatched request.
     * This is provided for use only on pending requests, such that when they timeout,
     * the user can recover the entire relayer fee.
     *
     * @notice Payment can be made with either the native token or the feeToken.
     * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
     * Will revert if enough native tokens are not provided.
     *
     * If no native tokens are provided then it will try to collect payment from the calling contract in
     * the feeToken.
     *
     * If called on an already delivered request, these funds will be seen as a donation to the hyperbridge protocol.
     * @param commitment - The request commitment
     * @param amount - The amount provided in `feeToken()`
     */
    function fundRequest(bytes32 commitment, uint256 amount) external payable notFrozen {
        if (msg.value > 0) {
            address[] memory path = new address[](2);
            address uniswapV2 = _hostParams.uniswapV2;
            path[0] = IUniswapV2Router02(uniswapV2).WETH();
            path[1] = feeToken();
            IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msg.value}(
                amount, path, address(this), block.timestamp
            );
        } else {
            IERC20(feeToken()).safeTransferFrom(_msgSender(), address(this), amount);
        }

        FeeMetadata memory metadata = _requestCommitments[commitment];
        if (metadata.sender == address(0)) revert UnknownRequest();

        metadata.fee += amount;
        _requestCommitments[commitment] = metadata;

        emit RequestFunded({commitment: commitment, newFee: metadata.fee});
    }

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
