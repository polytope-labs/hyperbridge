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
pragma solidity ^0.8.24;

import {IntentsBase} from "./intentsv2/IntentsBase.sol";
import {IntrinsicModule} from "./intentsv2/IntrinsicModule.sol";
import {ExtrinsicModule} from "./intentsv2/ExtrinsicModule.sol";

import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {ICallDispatcher, Call} from "@hyperbridge/core/interfaces/ICallDispatcher.sol";
import {IDispatcher} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ReentrancyGuardTransient} from "@openzeppelin/contracts/utils/ReentrancyGuardTransient.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {Ownable2StepUpgradeable} from "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {
    TokenInfo,
    Order,
    Params,
    InitParams,
    FillOptions,
    SelectOptions,
    CancelOptions,
    Deployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";

/**
 * @title IntentGatewayV2
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev Creates and fills same-chain and cross-chain orders. This is the proxy implementation: it
 * holds the entry points and shared fill steps, and delegatecalls route-specific logic to
 * `IntrinsicModule` (same-chain) and `ExtrinsicModule` (cross-chain) to stay under EIP-170.
 *
 * Governance upgrades the gateway and sets the relayer through `Execute`. The owner can only
 * pause and unpause it.
 */
contract IntentGatewayV2 is
    IntentsBase,
    HyperApp,
    ReentrancyGuardTransient,
    Initializable,
    Ownable2StepUpgradeable,
    PausableUpgradeable
{
    using SafeERC20 for IERC20;

    /// @dev Same-chain fills and cancels.
    address public immutable intrinsicModule;

    /// @dev Cross-chain fills, cancels and escrow settlement.
    address public immutable extrinsicModule;

    /// @dev The `Initializable` version this implementation lands a proxy on, through `initialize`
    /// or `migrate`. 3 is the module split, the owner and solver quotes, which land together.
    uint64 private constant VERSION = 3;

    /**
     * @dev Records the modules and locks this implementation against initialization. Modules must
     * have code: a delegatecall to an empty address succeeds and does nothing.
     */
    constructor(address intrinsic, address extrinsic) EIP712("IntentGateway", "2") {
        if (intrinsic.code.length == 0 || extrinsic.code.length == 0) revert InvalidInput();
        intrinsicModule = intrinsic;
        extrinsicModule = extrinsic;
        _disableInitializers();
    }

    /**
     * @dev Accepts native tokens, e.g. balances swept back from the CallDispatcher.
     */
    receive() external payable {}

    /**
     * @dev The Hyperbridge host.
     */
    function host() public view override(IntentsBase, HyperApp) returns (address) {
        return _params.host;
    }

    /**
     * @dev The initialized version: 0 on a bare proxy, `VERSION` once `initialize` or `migrate` has
     * run.
     */
    function version() external view returns (uint64) {
        return _getInitializedVersion();
    }

    /**
     * @dev The gateway's configuration.
     */
    function params() external view returns (Params memory) {
        return _params;
    }

    /**
     * @dev The gateway registered for `stateMachineId`. Reverts `UnknownInstance` if there is none.
     */
    function instance(bytes calldata stateMachineId) public view returns (address) {
        return _instance(stateMachineId);
    }

    /**
     * @dev Refuses an initialized proxy. Only the host-only `migrate` advances one.
     */
    modifier onlyFresh() {
        if (_getInitializedVersion() != 0) revert InvalidInitialization();
        _;
    }

    /**
     * @dev Initializes a bare proxy with its peers, params, relayer and owner.
     */
    function initialize(InitParams memory init) public onlyFresh reinitializer(VERSION) {
        uint256 peersLength = init.peerChains.length;
        for (uint256 i = 0; i < peersLength; i++) {
            Deployment memory deployment = Deployment({chain: init.peerChains[i], gateway: address(this)});
            _addDeployment(deployment);
        }
        _validateParams(init.params);
        _params = init.params;
        _setRelayer(init.relayer);
        __Ownable_init(init.owner);
        __Ownable2Step_init();
        __Pausable_init();
    }

    /**
     * @dev Takes a version-2 proxy to `VERSION`, as the init data of its upgrade. Moves `_relayer`
     * from slot 13 offset 1 to offset 0, dropping the removed `_paused` byte, and sets the owner.
     */
    function migrate(address owner_) external onlyHost reinitializer(VERSION) {
        assembly ("memory-safe") {
            sstore(_relayer.slot, shr(8, sload(_relayer.slot)))
        }
        __Ownable_init(owner_);
        __Ownable2Step_init();
        __Pausable_init();
    }

    /**
     * @dev Also accepts the host, so governance can pause, resume or replace the owner through
     * `Execute`.
     */
    function _checkOwner() internal view override {
        address sender = _msgSender();
        if (sender != owner() && sender != host()) revert OwnableUnauthorizedAccount(sender);
    }

    /**
     * @dev Pauses placement, fills and escrow deliveries. Cancels and governance still work.
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @dev Resumes the gateway.
     */
    function unpause() external onlyOwner {
        _unpause();
    }

    /**
     * @dev Escrows the caller's inputs and places the order.
     * Leg `i` sells `order.inputs[i]` for `order.output.assets[i]`, and every leg trades the same
     * pair, so an order is one pair quoted at one or more prices.
     *
     * The protocol fee comes out of each input before the commitment is computed.
     * @param order The order. `user`, `source` and `nonce` are overwritten.
     * @param graffiti Attribution tag emitted in `OrderPlaced`.
     */
    function placeOrder(Order memory order, bytes32 graffiti) public payable whenNotPaused nonReentrant {
        uint256 inputsLen = order.inputs.length;
        // Inputs and outputs pair 1:1 by index; a leg without its counterpart could never be filled.
        if (inputsLen == 0 || order.output.assets.length != inputsLen) revert InvalidInput();

        // Every leg trades the same pair. Tokens are read from their low 20 bytes, and anything above
        // would let one token pass the output sweep in `_execute` as two, so leg 0's are checked here
        // and the rest must equal leg 0 byte for byte, which rules out an alias of the same address.
        bytes32 inputToken = order.inputs[0].token;
        bytes32 outputToken = order.output.assets[0].token;
        if (uint256(inputToken) >> 160 != 0 || uint256(outputToken) >> 160 != 0) revert InvalidInput();

        for (uint256 i; i < inputsLen;) {
            if (order.inputs[i].token != inputToken) revert InvalidInput();
            if (order.output.assets[i].token != outputToken) revert InvalidInput();
            // A zero-amount output would strand its leg's escrow.
            if (order.output.assets[i].amount == 0) revert InvalidInput();
            unchecked {
                ++i;
            }
        }

        address hostAddr = host();
        order.user = bytes32(uint256(uint160(msg.sender)));
        order.source = IDispatcher(hostAddr).host();
        order.nonce = _nonce++;

        // Phase 1: Transfer tokens and record actual received amounts.
        // For fee-on-transfer tokens, the gateway receives less than the requested amount.
        // We mutate order.inputs to reflect actual received so the commitment and escrow
        // are consistent with what the gateway holds.
        uint256 msgValue = msg.value;
        if (order.predispatch.call.length > 0 && order.predispatch.assets.length > 0) {
            address dispatcher = _params.dispatcher;
            // Predispatch escrow is swept and measured per input token, so its legs must not share
            // one. Every leg holds the same token, so a predispatch order is single-leg.
            if (inputsLen != 1) revert InvalidInput();

            uint256 assetsLen = order.predispatch.assets.length;
            for (uint256 i; i < assetsLen;) {
                address token = address(uint160(uint256(order.predispatch.assets[i].token)));
                uint256 amount = order.predispatch.assets[i].amount;
                if (amount == 0) revert InvalidInput();

                if (token == address(0)) {
                    if (amount > msgValue) revert InsufficientNativeToken();
                    msgValue -= amount;

                    _sendValue(dispatcher, amount);
                } else {
                    IERC20(token).safeTransferFrom(msg.sender, dispatcher, amount);
                }

                unchecked {
                    ++i;
                }
            }

            ICallDispatcher(dispatcher).dispatch(order.predispatch.call);

            // Build sweep calls and snapshot gateway balances before the sweep.
            Call[] memory transferCalls = new Call[](inputsLen);
            uint256[] memory balancesBefore = new uint256[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                uint256 requiredAmount = order.inputs[i].amount;

                if (token == address(0)) {
                    uint256 balance = address(dispatcher).balance;
                    if (balance < requiredAmount) revert InsufficientNativeToken();
                    transferCalls[i] = Call({to: address(this), value: balance, data: ""});
                    balancesBefore[i] = address(this).balance;
                } else {
                    uint256 balance = IERC20(token).balanceOf(dispatcher);
                    if (balance < requiredAmount) revert InvalidInput();
                    transferCalls[i] = Call({
                        to: token,
                        value: 0,
                        data: abi.encodeWithSelector(IERC20.transfer.selector, address(this), balance)
                    });
                    balancesBefore[i] = IERC20(token).balanceOf(address(this));
                }

                unchecked {
                    ++i;
                }
            }

            ICallDispatcher(dispatcher).dispatch(abi.encode(transferCalls));

            // Measure actual received, emit dust for excess, update order.inputs.
            for (uint256 i; i < inputsLen;) {
                address token = address(uint160(uint256(order.inputs[i].token)));
                uint256 received;
                if (token == address(0)) {
                    received = address(this).balance - balancesBefore[i];
                } else {
                    received = IERC20(token).balanceOf(address(this)) - balancesBefore[i];
                }

                if (received > order.inputs[i].amount) {
                    uint256 dust = received - order.inputs[i].amount;
                    emit DustCollected(token, dust);
                } else {
                    order.inputs[i].amount = received;
                }

                unchecked {
                    ++i;
                }
            }
        } else {
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                if (token == address(0)) {
                    if (msgValue < order.inputs[i].amount) revert InsufficientNativeToken();
                    msgValue -= order.inputs[i].amount;
                } else {
                    uint256 balBefore = IERC20(token).balanceOf(address(this));
                    IERC20(token).safeTransferFrom(msg.sender, address(this), order.inputs[i].amount);
                    order.inputs[i].amount = IERC20(token).balanceOf(address(this)) - balBefore;
                }

                unchecked {
                    ++i;
                }
            }
        }

        // Phase 2: Compute protocol fees and commitment from actual received amounts.
        bytes32 destinationHash = keccak256(order.destination);
        uint256 protocolFeeBps = _destinationProtocolFees[destinationHash];
        if (protocolFeeBps == 0) {
            protocolFeeBps = _params.protocolFeeBps;
        }
        TokenInfo[] memory reducedInputs;
        uint256[] memory protocolFees = new uint256[](inputsLen);

        if (protocolFeeBps > 0) {
            reducedInputs = new TokenInfo[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                uint256 originalAmount = order.inputs[i].amount;
                if (originalAmount == 0) revert InvalidInput();
                uint256 protocolFee = (originalAmount * protocolFeeBps) / 10_000;
                uint256 reducedAmount = originalAmount - protocolFee;
                protocolFees[i] = protocolFee;

                reducedInputs[i] = TokenInfo({token: order.inputs[i].token, amount: reducedAmount});
                unchecked {
                    ++i;
                }
            }

            order.inputs = reducedInputs;
        } else {
            reducedInputs = order.inputs;
        }
        bytes32 commitment = keccak256(abi.encode(order));

        // Phase 3: Credit escrow, per leg.
        for (uint256 i; i < inputsLen;) {
            _orders[commitment][i] = reducedInputs[i].amount;
            uint256 fee = protocolFees[i];
            if (fee > 0) {
                _protocolFees[commitment][i] = ProtocolFee({amount: fee, committed: reducedInputs[i].amount});
            }

            unchecked {
                ++i;
            }
        }

        if (order.fees > 0) {
            address feeToken = IDispatcher(hostAddr).feeToken();
            if (msgValue > 0) {
                address uniswapV2 = IDispatcher(hostAddr).uniswapV2Router();
                address WETH = IUniswapV2Router02(uniswapV2).WETH();
                address[] memory path = new address[](2);
                path[0] = WETH;
                path[1] = feeToken;
                uint256[] memory amounts = IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msgValue}(
                    order.fees, path, address(this), block.timestamp
                );
                msgValue -= amounts[0];
            } else {
                IERC20(feeToken).safeTransferFrom(msg.sender, address(this), order.fees);
            }

            _orders[commitment][TRANSACTION_FEES] = order.fees;
        }

        // Refund any unspent native tokens to the user.
        if (msgValue > 0) {
            _sendValue(msg.sender, msgValue);
        }

        emit OrderPlaced({
            user: order.user,
            source: string(order.source),
            destination: string(order.destination),
            deadline: order.deadline,
            nonce: order.nonce,
            fees: order.fees,
            session: order.session,
            predispatch: order.predispatch.assets,
            inputs: reducedInputs,
            beneficiary: order.output.beneficiary,
            outputs: order.output.assets,
            predispatchCall: order.predispatch.call,
            outputCall: order.output.call,
            graffiti: graffiti
        });
    }

    /**
     * @dev Records a solver selection signed by the order's session key, for `fillOrder` in the
     * same transaction. Returns the session key. Reverts `Filled` on a finalized order.
     */
    function select(SelectOptions calldata options) public returns (address) {
        return _select(options);
    }

    /**
     * @dev Fills an order. The route's module pays the legs and settles the released escrow; the
     * rest of the fill happens here, the same for both routes.
     */
    function fillOrder(Order calldata order, FillOptions calldata options) public payable whenNotPaused nonReentrant {
        uint256 blockNumber = _blockNumber();
        if (order.deadline < blockNumber) revert Expired();
        if (options.validUntil != 0 && blockNumber > options.validUntil) revert FillExpired();
        bytes32 commitment = keccak256(abi.encode(order));

        address hostAddr = host();
        bytes32 currentChain = keccak256(IDispatcher(hostAddr).host());
        bytes32 orderSource = keccak256(order.source);
        bytes32 orderDest = keccak256(order.destination);
        bool isSameChain = orderSource == orderDest;

        if (isSameChain && orderSource != currentChain) revert WrongChain();
        if (!isSameChain && orderDest != currentChain) revert WrongChain();

        if (_filled[commitment] != address(0)) revert Filled();

        if (_params.solverSelection) {
            // The caller's own selection slot, so a second selection on this order in the same
            // bundle cannot clobber it. See `_select`.
            bytes32 selectionSlot = keccak256(abi.encode(commitment, msg.sender));
            bytes32 storedSelectionHash;
            assembly {
                storedSelectionHash := tload(selectionSlot)
            }

            bytes32 expectedSelectionHash = keccak256(abi.encode(order.session));
            if (storedSelectionHash != expectedSelectionHash) revert Unauthorized();
        }

        uint256 outputsLen = order.output.assets.length;
        if (options.outputs.length != outputsLen) revert InvalidInput();
        if (order.inputs.length != outputsLen) revert InvalidInput();
        if (options.inputs.length != outputsLen) revert InvalidInput();

        // Claimed for the whole fill; released below if the order stays open.
        _filled[commitment] = msg.sender;
        bytes memory returned = isSameChain
            ? _delegate(intrinsicModule, abi.encodeCall(IntrinsicModule.fillOrder, (order, options, commitment)))
            : _delegate(extrinsicModule, abi.encodeCall(ExtrinsicModule.fillOrder, (order, options, commitment)));
        FillResult memory result = abi.decode(returned, (FillResult));

        if (result.fullyFilled) {
            _execute(order);
            emit OrderFilled(commitment, msg.sender, result.creditedOutputs, result.releasedInputs);
        } else {
            delete _filled[commitment];
            emit PartialFill(commitment, msg.sender, result.creditedOutputs, result.releasedInputs);
        }

        if (result.nativeRemaining > 0) _sendValue(msg.sender, result.nativeRemaining);
    }

    /**
     * @dev Cancels an order. A same-chain order is refunded here, a cross-chain one on its source
     * chain after a Hyperbridge round trip. Only the creator may cancel before the deadline.
     */
    function cancelOrder(Order calldata order, CancelOptions calldata options) public payable nonReentrant {
        bytes32 commitment = keccak256(abi.encode(order));

        if (_filled[commitment] != address(0)) revert Filled();

        address hostAddr = host();
        bytes32 currentChain = keccak256(IDispatcher(hostAddr).host());
        bytes32 orderSource = keccak256(order.source);
        bytes32 orderDest = keccak256(order.destination);
        bool isSameChain = orderSource == orderDest;

        // Safe to emit early: a failed route reverts and discards the log.
        emit OrderCancelled({commitment: commitment, canceller: msg.sender});

        if (isSameChain) {
            // Checked here, where the chain ids are already known, rather than in the module.
            if (currentChain != orderSource) revert WrongChain();
            _delegate(intrinsicModule, abi.encodeCall(IntrinsicModule.cancelSameChain, (order, commitment)));
        } else if (currentChain == orderSource) {
            _delegate(extrinsicModule, abi.encodeCall(ExtrinsicModule.cancelFromSource, (order, options, commitment)));
        } else if (currentChain == orderDest) {
            _delegate(extrinsicModule, abi.encodeCall(ExtrinsicModule.cancelFromDest, (order, options, commitment)));
        } else {
            revert WrongChain();
        }
    }

    /**
     * @dev Forwards to the extrinsic module. While paused, only governance requests are accepted.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        bool isGovernance = keccak256(incoming.request.source) == keccak256(IDispatcher(host()).hyperbridge());
        if (paused() && !isGovernance) revert EnforcedPause();
        _delegate(extrinsicModule, msg.data);
    }

    /**
     * @dev Forwards to the extrinsic module. Reverts while paused.
     */
    function onGetResponse(IncomingGetResponse calldata) external override onlyHost whenNotPaused {
        _delegate(extrinsicModule, msg.data);
    }

    /**
     * @dev Delegatecalls `module` and returns its return data. Reverts are re-raised byte for byte,
     * so custom error selectors survive.
     */
    function _delegate(address module, bytes memory data) internal returns (bytes memory returned) {
        assembly ("memory-safe") {
            let ok := delegatecall(gas(), module, add(data, 0x20), mload(data), 0, 0)
            let size := returndatasize()
            returned := mload(0x40)
            returndatacopy(add(returned, 0x20), 0, size)
            if iszero(ok) { revert(add(returned, 0x20), size) }
            mstore(returned, size)
            mstore(0x40, add(add(returned, 0x20), and(add(size, 0x1f), not(0x1f))))
        }
    }
}
