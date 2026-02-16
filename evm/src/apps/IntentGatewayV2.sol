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

import {DispatchPost, DispatchGet, IDispatcher, PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IncomingPostRequest, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {
    PaymentInfo,
    TokenInfo,
    DispatchInfo,
    Order,
    SweepDust,
    Params,
    ParamsUpdate,
    DestinationFee,
    WithdrawalRequest,
    FillOptions,
    SelectOptions,
    CancelOptions,
    NewDeployment
} from "@hyperbridge/core/apps/IntentGatewayV2.sol";
import {IIntentPriceOracle} from "@hyperbridge/core/apps/IntentPriceOracle.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";

import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {ICallDispatcher, Call} from "../interfaces/ICallDispatcher.sol";

/**
 * @title IntentGatewayV2
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev The IntentGateway allows for the creation and fulfillment of same-chain & cross-chain orders.
 */
contract IntentGatewayV2 is HyperApp, EIP712 {
    using SafeERC20 for IERC20;

    /**
     * @dev EIP-712 type hash for SelectSolver message
     */
    bytes32 public constant SELECT_SOLVER_TYPEHASH = keccak256("SelectSolver(bytes32 commitment,address solver)");

    /**
     * @dev Enum representing the different kinds of incoming requests that can be executed.
     */
    enum RequestKind {
        /// @dev Identifies a request for redeeming an escrow.
        RedeemEscrow,
        /// @dev Identifies a request for recording new contract deployments
        NewDeployment,
        /// @dev Identifies a request for updating parameters.
        UpdateParams,
        /// @dev Identifies a request for sweeping accumulated dust
        SweepDust,
        /// @dev Identifies a request for refunding an escrow (cancellation from destination chain)
        RefundEscrow
    }

    /**
     * @dev Address constant for transaction fees, derived from the keccak256 hash of the string "txFees".
     * This address is used to store or reference the transaction fees within the contract.
     */
    address private constant TRANSACTION_FEES = address(uint160(uint256(keccak256("txFees"))));

    /**
     * @notice Constant representing a filled slot in big endian format
     * @dev Hex value 0x06 padded with leading zeros to fill 32 bytes
     */
    bytes32 constant FILLED_SLOT_BIG_ENDIAN_BYTES =
        hex"0000000000000000000000000000000000000000000000000000000000000002";

    /**
     * @dev Mapping to store the addresses associated with filled intents.
     * The key is a bytes32 hash representing the intent, and the value is the address
     * that filled the intent.
     */
    mapping(bytes32 => address) public _filled;

    /**
     * @dev Private variable to store the nonce value.
     * This nonce is used to ensure the uniqueness of orders.
     */
    uint256 public _nonce;

    /**
     * @dev Private variable to store the parameters for the IntentGateway module.
     * This variable is of type `Params` and is used internally within the contract.
     */
    Params private _params;

    /**
     * @dev Address of the admin, which can initialize the contract.
     * The admin is reset to the zero address after initialization.
     */
    address private _admin;

    /**
     * @dev Mapping to store orders.
     * The outer mapping key is a bytes32 value representing the order commitment.
     * The inner mapping key is an address representing the escrowed token contract.
     * The inner mapping value is a uint256 representing the order amount.
     */
    mapping(bytes32 => mapping(address => uint256)) public _orders;

    /**
     * @dev Mapping to store instances of contracts.
     * The key the keccak(stateMachineId) and the value is the address of a known contract instance.
     */
    mapping(bytes32 => address) public _instances;

    /**
     * @dev Mapping to store destination-specific protocol fees.
     * The key is keccak256(stateMachineId) and the value is the protocol fee in basis points.
     * If the value is 0, falls back to the source chain protocol fee from _params.protocolFeeBps.
     */
    mapping(bytes32 => uint256) public _destinationProtocolFees;

    /// @notice Thrown when an unauthorized action is attempted.
    error Unauthorized();

    /// @notice Thrown when an invalid input is provided.
    error InvalidInput();

    /// @notice Thrown when an action is attempted on an expired order.
    error Expired();

    /// @notice Thrown when there are insufficient native tokens to complete an action.
    error InsufficientNativeToken();

    /// @notice Thrown when an action is attempted on an order that has not yet expired.
    error NotExpired();

    /// @notice Thrown when an action is attempted on an order that has already been filled.
    error Filled();

    /// @notice Thrown when an action is attempted on an order that has been cancelled.
    error Cancelled();

    /// @notice Thrown when an action is attempted on the wrong chain.
    error WrongChain();

    /// @notice Thrown when an action is attempted on an unknown order.
    error UnknownOrder();

    /**
     * @dev Emitted when an order is placed.
     */
    event OrderPlaced(
        /// @dev The address of the user who is initiating the transfer
        bytes32 user,
        /// @dev The state machine identifier of the origin chain
        bytes source,
        /// @dev The state machine identifier of the destination chain
        bytes destination,
        /// @dev The block number by which the order must be filled on the destination chain
        uint256 deadline,
        /// @dev The nonce of the order
        uint256 nonce,
        /// @dev Represents the dispatch fees associated with the IntentGateway.
        uint256 fees,
        /// @dev Optional session key used to select winning solver.
        address session,
        /// @dev Asset beneficiary on the destination chain
        bytes32 beneficiary,
        /// @dev Assets that were used to execute a predispatch call
        TokenInfo[] predispatch,
        /// @dev The tokens that are escrowed for the filler.
        TokenInfo[] inputs,
        /// @dev The tokens that the filler will provide.
        TokenInfo[] outputs
    );

    /**
     * @dev Emitted when an order is filled.
     * @param commitment The unique identifier of the order.
     * @param filler The address of the entity that filled the order.
     */
    event OrderFilled(bytes32 indexed commitment, address filler);

    /**
     * @dev Emitted when an escrow is released.
     * @param commitment The unique identifier of the order.
     */
    event EscrowReleased(bytes32 indexed commitment);

    /**
     * @dev Emitted when an escrow is refunded.
     * @param commitment The unique identifier of the order.
     */
    event EscrowRefunded(bytes32 indexed commitment);

    /**
     * @dev Emitted when the parameters are updated.
     * @param previous The previous parameters.
     * @param current The current parameters.
     */
    event ParamsUpdated(Params previous, Params current);

    /**
     * @dev Emitted when a new deployment of IntentGateway is added.
     * @param stateMachineId The identifier for the state machine.
     * @param gateway The gateway identifier.
     */
    event NewDeploymentAdded(bytes stateMachineId, address gateway);

    /**
     * @dev Emitted when some dust tokens are accrued.
     * @param token The token contract address of the dust, address(0) for native currency.
     * @param amount The amount of dust collected.
     */
    event DustCollected(address token, uint256 amount);

    /**
     * @dev Emitted when some dust tokens are swept.
     * @param token The token contract address of the fee, address(0) for native currency.
     * @param amount The amount of dust to be swept.
     * @param beneficiary The beneficiary of the funds
     */
    event DustSwept(address token, uint256 amount, address beneficiary);

    /**
     * @dev Emitted when a destination-specific protocol fee is updated.
     * @param stateMachineId The hashed state machine identifier of the destination chain.
     * @param feeBps The protocol fee in basis points for this destination.
     */
    event DestinationProtocolFeeUpdated(bytes32 indexed stateMachineId, uint256 feeBps);

    constructor(address admin) EIP712("IntentGateway", "2") {
        _admin = admin;
    }

    /**
     * @notice Fallback function to receive ether
     * @dev This function is called when ether is sent to the contract without data
     * @custom:note The function is marked payable to allow receiving ether
     */
    receive() external payable {}

    /**
     * @dev Should return the `IsmpHost` address for the current chain.
     * The `IsmpHost` is an immutable contract that will never change.
     */
    function host() public view override returns (address) {
        return _params.host;
    }

    /**
     * @notice Returns the EIP-712 domain separator
     * @return bytes32 The domain separator used for EIP-712 signatures
     */
    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return _domainSeparatorV4();
    }

    /**
     * @dev Fetch the IntentGateway contract instance for a chain.
     */
    function instance(bytes calldata stateMachineId) public view returns (address) {
        address gateway = _instances[keccak256(stateMachineId)];
        return gateway == address(0) ? address(this) : gateway;
    }

    /**
     * @dev Checks that the request originates from a known instance of the IntentGateway.
     */
    function authenticate(PostRequest calldata request) internal view {
        if (request.from.length != 20) revert InvalidInput();
        address module = address(bytes20(request.from));
        // IntentGateway only accepts incoming assets from itself or known instances
        if (instance(request.source) != module) revert Unauthorized();
    }

    /**
     * @notice Sets the parameters for the IntentGateway.
     * @param p The parameters to be set, encapsulated in a Params struct.
     */
    function setParams(Params memory p) public {
        if (msg.sender != _admin) revert Unauthorized();

        _admin = address(0);
        _params = p;
    }

    /**
     * @notice Retrieves the current parameter settings for the IntentGateway module
     * @dev Returns a struct containing all configurable parameters
     * @return Params A struct containing the module's current parameters
     */
    function params() external view returns (Params memory) {
        return _params;
    }

    /**
     * @notice Calculates the commitment slot hash required for storage queries.
     * @dev The commitment slot hash is used as part of the proof verification process
     * @param commitment The commitment value as a bytes32 hash
     * @return bytes The calculated commitment slot hash
     */
    function calculateCommitmentSlotHash(bytes32 commitment) public pure returns (bytes memory) {
        return abi.encodePacked(keccak256(abi.encodePacked(commitment, FILLED_SLOT_BIG_ENDIAN_BYTES)));
    }

    /**
     * @notice Places an order with the given order details.
     * @dev This function allows users to place an order by providing the order details.
     * @param order The order details to be placed.
     * @param graffiti The arbitrary data used for identification purposes.
     */
    function placeOrder(Order memory order, bytes32 graffiti) public payable {
        // Validate that order has inputs
        if (order.inputs.length == 0) revert InvalidInput();

        address hostAddr = host();
        // fill out the order preludes
        order.user = bytes32(uint256(uint160(msg.sender)));
        order.source = IDispatcher(hostAddr).host();
        order.nonce = _nonce++;

        // Calculate reduced inputs (after protocol fees) for commitment and escrow
        uint256 inputsLen = order.inputs.length;
        // Use destination-specific protocol fee, fallback to source chain fee if zero
        bytes32 destinationHash = keccak256(order.destination);
        uint256 protocolFeeBps = _destinationProtocolFees[destinationHash];
        if (protocolFeeBps == 0) {
            protocolFeeBps = _params.protocolFeeBps;
        }
        TokenInfo[] memory reducedInputs;
        bytes32 commitment;

        if (protocolFeeBps > 0) {
            reducedInputs = new TokenInfo[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                uint256 originalAmount = order.inputs[i].amount;
                uint256 protocolFee = (originalAmount * protocolFeeBps) / 10_000;
                uint256 reducedAmount = originalAmount - protocolFee;
                address token = address(uint160(uint256(order.inputs[i].token)));

                // Emit DustCollected for protocol fee if non-zero
                if (protocolFee > 0) emit DustCollected(token, protocolFee);

                reducedInputs[i] = TokenInfo({token: order.inputs[i].token, amount: reducedAmount});
                unchecked {
                    ++i;
                }
            }

            // Temporarily swap inputs to calculate commitment with reduced amounts
            TokenInfo[] memory originalInputs = order.inputs;
            order.inputs = reducedInputs;
            commitment = keccak256(abi.encode(order));
            order.inputs = originalInputs;
        } else {
            // No protocol fees, use order.inputs directly
            reducedInputs = order.inputs;
            commitment = keccak256(abi.encode(order));
        }

        // escrow tokens
        uint256 msgValue = msg.value;
        if (order.predispatch.call.length > 0 && order.predispatch.assets.length > 0) {
            address dispatcher = _params.dispatcher;

            // Transfer all predispatch assets to the call dispatcher
            uint256 assetsLen = order.predispatch.assets.length;
            for (uint256 i; i < assetsLen;) {
                address token = address(uint160(uint256(order.predispatch.assets[i].token)));
                uint256 amount = order.predispatch.assets[i].amount;

                if (token == address(0)) {
                    if (amount > msgValue) revert InsufficientNativeToken();
                    msgValue -= amount;

                    (bool sent,) = dispatcher.call{value: amount}("");
                    if (!sent) revert InsufficientNativeToken();
                } else {
                    IERC20(token).safeTransferFrom(msg.sender, dispatcher, amount);
                }

                unchecked {
                    ++i;
                }
            }

            // Execute the call dispatcher with predispatch call
            ICallDispatcher(dispatcher).dispatch(order.predispatch.call);

            // Transfer tokens from call dispatcher back to IntentGateway
            Call[] memory transferCalls = new Call[](inputsLen);
            for (uint256 i; i < inputsLen;) {
                address token = address(uint160(uint256(order.inputs[i].token)));
                uint256 requiredAmount = order.inputs[i].amount;
                uint256 balance;

                if (token == address(0)) {
                    balance = address(dispatcher).balance;
                    if (balance < requiredAmount) revert InsufficientNativeToken();
                    transferCalls[i] = Call({to: address(this), value: balance, data: ""});
                } else {
                    balance = IERC20(token).balanceOf(dispatcher);
                    if (balance < requiredAmount) revert InvalidInput();
                    transferCalls[i] = Call({
                        to: token,
                        value: 0,
                        data: abi.encodeWithSelector(IERC20.transfer.selector, address(this), balance)
                    });
                }

                uint256 dust = balance - requiredAmount;
                if (dust > 0) emit DustCollected(token, dust);

                // Store reduced amount (after protocol fees) in escrow
                _orders[commitment][token] += reducedInputs[i].amount;

                unchecked {
                    ++i;
                }
            }

            // Execute transfer calls from call dispatcher
            ICallDispatcher(dispatcher).dispatch(abi.encode(transferCalls));
        } else {
            for (uint256 i; i < inputsLen;) {
                if (order.inputs[i].amount == 0) revert InvalidInput();
                address token = address(uint160(uint256(order.inputs[i].token)));
                if (token == address(0)) {
                    // native token
                    if (msgValue < order.inputs[i].amount) revert InsufficientNativeToken();
                    msgValue -= order.inputs[i].amount;
                } else {
                    IERC20(token).safeTransferFrom(msg.sender, address(this), order.inputs[i].amount);
                }

                // Store reduced amount (after protocol fees) in escrow
                _orders[commitment][token] += reducedInputs[i].amount;

                unchecked {
                    ++i;
                }
            }
        }

        if (order.fees > 0) {
            // escrow fees
            address feeToken = IDispatcher(hostAddr).feeToken();
            if (msgValue > 0) {
                address uniswapV2 = IDispatcher(hostAddr).uniswapV2Router();
                address WETH = IUniswapV2Router02(uniswapV2).WETH();
                address[] memory path = new address[](2);
                path[0] = WETH;
                path[1] = IDispatcher(hostAddr).feeToken();
                IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msgValue}(
                    order.fees, path, address(this), block.timestamp
                );
            } else {
                IERC20(feeToken).safeTransferFrom(msg.sender, address(this), order.fees);
            }

            _orders[commitment][TRANSACTION_FEES] = order.fees;
        }

        emit OrderPlaced({
            user: order.user,
            source: order.source,
            destination: order.destination,
            deadline: order.deadline,
            nonce: order.nonce,
            fees: order.fees,
            session: order.session,
            predispatch: order.predispatch.assets,
            inputs: reducedInputs,
            beneficiary: order.output.beneficiary,
            outputs: order.output.assets
        });
    }

    /**
     * @dev Selects a solver for an order. Should be called in the same transaction as `fillOrder`.
     * @param options The options for selecting a solver.
     */
    function select(SelectOptions calldata options) public returns (address) {
        // Verify that the session key signed (commitment, options.solver) using EIP-712
        bytes32 structHash = keccak256(abi.encode(SELECT_SOLVER_TYPEHASH, options.commitment, options.solver));
        bytes32 digest = _hashTypedDataV4(structHash);
        address sessionKey = ECDSA.recover(digest, options.signature);

        // store some preludes
        bytes32 commitment = options.commitment;
        bytes32 solver = bytes32(uint256(uint160(options.solver)));
        bytes32 sessionKeyBytes = bytes32(uint256(uint160(sessionKey)));
        bytes32 sessionSlot = bytes32(uint256(commitment) + 1);
        assembly {
            tstore(commitment, solver)
            tstore(sessionSlot, sessionKeyBytes)
        }

        return sessionKey;
    }

    /**
     * @notice Fills an order with the specified options.
     * @param order The order to be filled.
     * @param options The options to be used when filling the order.
     * @dev This function is payable and can accept Ether.
     */
    function fillOrder(Order calldata order, FillOptions calldata options) public payable {
        // Ensure the order has not expired
        if (order.deadline < block.number) revert Expired();
        bytes32 commitment = keccak256(abi.encode(order));

        address hostAddr = host();
        bytes32 currentChain = keccak256(IDispatcher(hostAddr).host());
        bytes32 orderSource = keccak256(order.source);
        bytes32 orderDest = keccak256(order.destination);
        bool isSameChain = orderSource == orderDest;

        // For same-chain swaps, must be on the source chain
        if (isSameChain && orderSource != currentChain) revert WrongChain();
        // For cross-chain, must be on the destination chain
        if (!isSameChain && orderDest != currentChain) revert WrongChain();

        if (_params.solverSelection) {
            // Verify solver selection
            bytes32 solver;
            bytes32 storedSessionKey;
            bytes32 sessionSlot = bytes32(uint256(commitment) + 1);
            assembly {
                solver := tload(commitment)
                storedSessionKey := tload(sessionSlot)
            }

            if (address(uint160(uint256(solver))) != msg.sender) revert Unauthorized();
            if (address(uint160(uint256(storedSessionKey))) != order.session) revert Unauthorized();
        }

        // Ensure the order has not been filled
        if (_filled[commitment] != address(0)) revert Filled();

        // Validate that solver outputs are provided and match order outputs length
        uint256 outputsLen = order.output.assets.length;
        if (options.outputs.length != outputsLen) revert InvalidInput();

        // no sneaky replay attacks
        _filled[commitment] = msg.sender;

        // fill the order
        uint256 msgValue = msg.value;
        address beneficiary = address(uint160(uint256(order.output.beneficiary)));
        for (uint256 i; i < outputsLen;) {
            address token = address(uint160(uint256(order.output.assets[i].token)));
            uint256 requestedAmount = order.output.assets[i].amount;

            if (options.outputs[i].token != order.output.assets[i].token) revert InvalidInput();

            uint256 solverAmount = options.outputs[i].amount;
            if (solverAmount < requestedAmount) revert InvalidInput();

            uint256 dust = solverAmount - requestedAmount;
            uint256 beneficiaryShare = 0;
            uint256 protocolShare = 0;

            if (dust > 0) {
                if (order.output.call.length > 0) {
                    protocolShare = dust;
                } else {
                    protocolShare = (dust * _params.surplusShareBps) / 10_000;
                    beneficiaryShare = dust - protocolShare;
                }
            }

            if (token == address(0)) {
                if (msgValue < solverAmount) revert InsufficientNativeToken();

                uint256 beneficiaryTotal = requestedAmount + beneficiaryShare;
                (bool sent,) = beneficiary.call{value: beneficiaryTotal}("");
                if (!sent) revert InsufficientNativeToken();

                msgValue -= beneficiaryTotal;
            } else {
                IERC20(token).safeTransferFrom(msg.sender, beneficiary, requestedAmount + beneficiaryShare);

                if (protocolShare > 0) {
                    IERC20(token).safeTransferFrom(msg.sender, address(this), protocolShare);
                }
            }

            if (protocolShare > 0) emit DustCollected(token, protocolShare);

            unchecked {
                ++i;
            }
        }

        if (order.output.call.length > 0) {
            address dispatcher = _params.dispatcher;

            ICallDispatcher(dispatcher).dispatch(order.output.call);

            // Sweep any tokens left in the dispatcher after execution
            uint256 assetsLen = order.output.assets.length;
            Call[] memory sweepCalls = new Call[](assetsLen);
            uint256 sweepCount = 0;

            for (uint256 i; i < assetsLen;) {
                address token = address(uint160(uint256(order.output.assets[i].token)));

                if (token == address(0)) {
                    // Native token
                    uint256 balance = dispatcher.balance;
                    if (balance > 0) {
                        sweepCalls[sweepCount] = Call({to: address(this), value: balance, data: ""});
                        sweepCount++;
                        emit DustCollected(token, balance);
                    }
                } else {
                    uint256 balance = IERC20(token).balanceOf(dispatcher);
                    if (balance > 0) {
                        sweepCalls[sweepCount] = Call({
                            to: token,
                            value: 0,
                            data: abi.encodeWithSelector(IERC20.transfer.selector, address(this), balance)
                        });
                        sweepCount++;
                        emit DustCollected(token, balance);
                    }
                }

                unchecked {
                    ++i;
                }
            }

            if (sweepCount > 0) {
                Call[] memory finalCalls = new Call[](sweepCount);
                for (uint256 i; i < sweepCount;) {
                    finalCalls[i] = sweepCalls[i];
                    unchecked {
                        ++i;
                    }
                }
                ICallDispatcher(dispatcher).dispatch(abi.encode(finalCalls));
            }
        }

        if (isSameChain) {
            // Same-chain swap: release escrow immediately
            WithdrawalRequest memory body = WithdrawalRequest({
                commitment: commitment, tokens: order.inputs, beneficiary: bytes32(uint256(uint160(msg.sender)))
            });
            withdraw(body, false);
        } else {
            // Cross-chain swap: dispatch settlement message
            bytes memory body = bytes.concat(
                bytes1(uint8(RequestKind.RedeemEscrow)),
                abi.encode(
                    WithdrawalRequest({
                        commitment: commitment, tokens: order.inputs, beneficiary: bytes32(uint256(uint160(msg.sender)))
                    })
                )
            );
            DispatchPost memory request = DispatchPost({
                dest: order.source,
                to: abi.encodePacked(instance(order.source)),
                body: body,
                timeout: 0,
                fee: options.relayerFee,
                payer: msg.sender
            });

            // dispatch settlement message
            if (options.nativeDispatchFee > 0 && msgValue >= options.nativeDispatchFee) {
                // there's some native tokens left to pay for request dispatch
                IDispatcher(hostAddr).dispatch{value: options.nativeDispatchFee}(request);
            } else {
                // try to pay for dispatch with fee token
                dispatchWithFeeToken(request, msg.sender);
            }
        }

        // Record spread with price oracle if configured
        if (_params.priceOracle != address(0)) {
            IIntentPriceOracle(_params.priceOracle)
                .recordSpread(commitment, order.source, order.inputs, options.outputs);
        }

        emit OrderFilled({commitment: commitment, filler: msg.sender});
    }

    /**
     * @notice Cancels an existing order.
     * @param order The order to be canceled.
     * @param options Additional options for the cancellation process.
     * @dev This function can only be called by the order owner and requires a payment.
     * It will initiate a storage query on the source chain to verify the order has not
     * yet been filled. If the order has not been filled, the tokens will be released.
     */
    function cancelOrder(Order calldata order, CancelOptions calldata options) public payable {
        bytes32 commitment = keccak256(abi.encode(order));

        // order has already been filled
        if (_filled[commitment] != address(0)) revert Filled();

        address hostAddr = host();
        bytes32 currentChain = keccak256(IDispatcher(hostAddr).host());
        bytes32 orderSource = keccak256(order.source);
        bytes32 orderDest = keccak256(order.destination);
        bool isSameChain = orderSource == orderDest;

        if (isSameChain) {
            // Same-chain: validate locally and refund immediately
            // only owner can cancel
            if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();

            // Verify we're on the correct chain
            if (orderSource != currentChain) revert WrongChain();

            WithdrawalRequest memory body =
                WithdrawalRequest({commitment: commitment, tokens: order.inputs, beneficiary: order.user});

            withdraw(body, true);
        } else if (currentChain == orderSource) {
            // source chain: fetch storage proof from destination
            // only owner can cancel order
            if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();

            // order has not yet expired
            if (options.height <= order.deadline) revert NotExpired();

            // Cross-chain: fetch storage proof
            uint256 inputsLen = order.inputs.length;
            for (uint256 i; i < inputsLen;) {
                // check for order existence
                if (_orders[commitment][address(uint160(uint256(order.inputs[i].token)))] == 0) revert UnknownOrder();

                unchecked {
                    ++i;
                }
            }

            bytes memory context =
                abi.encode(WithdrawalRequest({commitment: commitment, tokens: order.inputs, beneficiary: order.user}));

            bytes[] memory keys = new bytes[](1);
            keys[0] = bytes.concat(
                // contract address
                abi.encodePacked(instance(order.destination)),
                // storage slot hash
                calculateCommitmentSlotHash(commitment)
            );
            DispatchGet memory request = DispatchGet({
                dest: order.destination,
                keys: keys,
                timeout: 0,
                height: uint64(options.height),
                fee: options.relayerFee,
                context: context
            });

            // dispatch storage query request
            if (msg.value > 0) {
                // there's some native tokens left to pay for request dispatch
                IDispatcher(hostAddr).dispatch{value: msg.value}(request);
            } else {
                // try to pay for dispatch with fee token
                dispatchWithFeeToken(request, msg.sender);
            }
        } else if (currentChain == orderDest) {
            // destination chain: dispatch RefundEscrow request to source chain
            // If order hasn't expired, only owner can cancel
            if (order.deadline >= block.number) {
                if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();
            }

            // Mark as cancelled locally to prevent fills
            _filled[commitment] = address(uint160(uint256(order.user)));

            bytes memory body = bytes.concat(
                bytes1(uint8(RequestKind.RefundEscrow)),
                abi.encode(WithdrawalRequest({commitment: commitment, tokens: order.inputs, beneficiary: order.user}))
            );

            DispatchPost memory request = DispatchPost({
                dest: order.source,
                to: abi.encodePacked(instance(order.source)),
                body: body,
                timeout: 0,
                fee: options.relayerFee,
                payer: msg.sender
            });

            // dispatch refund request
            if (msg.value > 0) {
                IDispatcher(hostAddr).dispatch{value: msg.value}(request);
            } else {
                // try to pay for dispatch with fee token
                dispatchWithFeeToken(request, msg.sender);
            }
        } else {
            revert WrongChain();
        }
    }

    /**
     * @notice Executes an incoming post request.
     * @dev This function is called when an incoming post request is accepted.
     * It is only accessible by the host.
     * @param incoming The incoming post request data.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        if (kind == RequestKind.RedeemEscrow || kind == RequestKind.RefundEscrow) {
            authenticate(incoming.request);
            WithdrawalRequest memory body = abi.decode(incoming.request.body[1:], (WithdrawalRequest));
            return withdraw(body, kind == RequestKind.RefundEscrow);
        }

        // only hyperbridge is permitted to perfom these actions
        if (keccak256(incoming.request.source) != keccak256(IDispatcher(host()).hyperbridge())) revert Unauthorized();
        if (kind == RequestKind.NewDeployment) {
            NewDeployment memory body = abi.decode(incoming.request.body[1:], (NewDeployment));
            _instances[keccak256(body.stateMachineId)] = body.gateway;

            emit NewDeploymentAdded({stateMachineId: body.stateMachineId, gateway: body.gateway});
        } else if (kind == RequestKind.UpdateParams) {
            // Decode the body which includes optional destination-specific protocol fee updates
            ParamsUpdate memory update = abi.decode(incoming.request.body[1:], (ParamsUpdate));
            emit ParamsUpdated({previous: _params, current: update.params});
            _params = update.params;

            // Update destination-specific protocol fees if provided
            for (uint256 i; i < update.destinationFees.length;) {
                bytes32 stateMachineId = update.destinationFees[i].stateMachineId;
                uint256 feeBps = update.destinationFees[i].destinationFeeBps;
                _destinationProtocolFees[stateMachineId] = feeBps;

                unchecked {
                    ++i;
                }
                emit DestinationProtocolFeeUpdated(stateMachineId, feeBps);
            }
        } else if (kind == RequestKind.SweepDust) {
            SweepDust memory req = abi.decode(incoming.request.body[1:], (SweepDust));

            uint256 outputsLen = req.outputs.length;
            for (uint256 i; i < outputsLen;) {
                TokenInfo memory info = req.outputs[i];
                address token = address(uint160(uint256(info.token)));
                uint256 amount = info.amount;

                if (token == address(0)) {
                    (bool sent,) = req.beneficiary.call{value: amount}("");
                    if (!sent) revert InsufficientNativeToken();
                } else {
                    IERC20(token).safeTransfer(req.beneficiary, amount);
                }
                unchecked {
                    ++i;
                }
                emit DustSwept(token, amount, req.beneficiary);
            }
        }
    }

    /**
     * @notice Withdraws the escrowed tokens for a request body.
     * @dev This function is marked as internal.
     * @param body The request body containing commitment, tokens, and beneficiary.
     * @param isRefund Whether this is a refund (true) or a successful fill (false).
     */
    function withdraw(WithdrawalRequest memory body, bool isRefund) internal {
        address beneficiary = address(uint160(uint256(body.beneficiary)));
        _filled[body.commitment] = beneficiary;

        // redeem escrowed tokens
        uint256 len = body.tokens.length;
        for (uint256 i; i < len;) {
            address token = address(uint160(uint256(body.tokens[i].token)));
            uint256 amount = body.tokens[i].amount;
            if (_orders[body.commitment][token] == 0) revert UnknownOrder();

            if (token == address(0)) {
                (bool sent,) = beneficiary.call{value: amount}("");
                if (!sent) revert InsufficientNativeToken();
            } else {
                IERC20(token).safeTransfer(beneficiary, amount);
            }

            _orders[body.commitment][token] -= amount;
            unchecked {
                ++i;
            }
        }

        // redeem tx fees
        uint256 fees = _orders[body.commitment][TRANSACTION_FEES];
        if (fees > 0) {
            IERC20(IDispatcher(host()).feeToken()).safeTransfer(beneficiary, fees);
            delete _orders[body.commitment][TRANSACTION_FEES];
        }

        if (isRefund) {
            emit EscrowRefunded({commitment: body.commitment});
        } else {
            emit EscrowReleased({commitment: body.commitment});
        }
    }

    /**
     * @notice Handles the response for a previously dispatched storage query (GET request).
     * @dev This function is called by the host to process the response of a GET request.
     * @param incoming The response data structure for the GET request.
     * Only the host can call this function.
     */
    function onGetResponse(IncomingGetResponse calldata incoming) external override onlyHost {
        if (incoming.response.values[0].value.length != 0) revert Filled();

        WithdrawalRequest memory body = abi.decode(incoming.response.request.context, (WithdrawalRequest));
        withdraw(body, true);
    }
}
