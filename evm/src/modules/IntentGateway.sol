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

import {DispatchPost, DispatchGet, IDispatcher, PostRequest} from "@polytope-labs/ismp-solidity-v1/IDispatcher.sol";
import {BaseIsmpModule, IncomingPostRequest, IncomingGetResponse} from "@polytope-labs/ismp-solidity-v1/IIsmpModule.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity-v1/StateMachine.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";

import {ICallDispatcher} from "../interfaces/ICallDispatcher.sol";

/**
 * @notice Tokens that must be received for a valid order fulfillment
 */
struct PaymentInfo {
    /// @dev The address of the ERC20 token on the destination chain
    /// @dev address(0) used as a sentinel for the native token
    bytes32 token;
    /// @dev The amount of the token to be sent
    uint256 amount;
    /// @dev The address to receive the output tokens
    bytes32 beneficiary;
}

/**
 * @notice Tokens that must be escrowed for an order
 */
struct TokenInfo {
    /// @dev The address of the ERC20 token on the destination chain
    /// @dev address(0) used as a sentinel for the native token
    bytes32 token;
    /// @dev The amount of the token to be sent
    uint256 amount;
}

/**
 * @dev Represents an order in the IntentGateway module.
 * @param Order The structure defining an order.
 */
struct Order {
    /// @dev The address of the user who is initiating the transfer
    bytes32 user;
    /// @dev The state machine identifier of the origin chain
    bytes sourceChain;
    /// @dev The state machine identifier of the destination chain
    bytes destChain;
    /// @dev The block number by which the order must be filled on the destination chain
    uint256 deadline;
    /// @dev The nonce of the order
    uint256 nonce;
    /// @dev Represents the dispatch fees associated with the IntentGateway.
    uint256 fees;
    /// @dev The tokens that the filler will provide.
    PaymentInfo[] outputs;
    /// @dev The tokens that are escrowed for the filler.
    TokenInfo[] inputs;
    /// @dev A bytes array to store the calls if any.
    bytes callData;
}

/**
 * @dev Struct to define the parameters for the IntentGateway module.
 */
struct Params {
    /// @dev The address of the host contract
    address host;
    /// @dev Address of the dispatcher contract responsible for handling intents.
    address dispatcher;
}

/**
 * @dev Struct representing the body of a request.
 */
struct RequestBody {
    /// @dev Represents the commitment of an order. This is typically a hash that uniquely identifies the order.
    bytes32 commitment;
    /// @dev Stores the identifier for the beneficiary.
    bytes32 beneficiary;
    /// @dev An array of token identifiers. Each element in the array represents a unique token involved in the order.
    TokenInfo[] tokens;
}

/**
 * @notice A struct representing the options for filling an intent.
 * @dev This struct is used to specify various parameters and options
 *      when filling an intent in the IntentGateway contract.
 */
struct FillOptions {
    /// @dev The fee paid to the relayer for processing transactions.
    uint256 relayerFee;
}

/**
 * @dev Struct representing the options for canceling an intent.
 */
struct CancelOptions {
    /// @dev The fee paid to the relayer for processing transactions.
    uint256 relayerFee;
    /// @dev Stores the height value.
    uint256 height;
}

/**
 * @dev Struct representing a new instance of IntentGateway.
 */
struct NewDeployment {
    /// @dev Identifier for the state machine.
    bytes stateMachineId;
    /// @dev A bytes32 variable to store the gateway identifier.
    bytes32 gateway;
}

/**
 * @title IntentGateway
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @dev The IntentGateway allows for the creation and fulfillment of cross-chain orders.
 */
contract IntentGateway is BaseIsmpModule {
    using SafeERC20 for IERC20;

    /**
     * @dev Enum representing the different kinds of incoming requests that can be executed.
     */
    enum RequestKind {
        /// @dev Identifies a request for redeeming an escrow.
        RedeemEscrow,
        /// @dev Identifies a request for recording new contract deployments
        NewDeployment,
        /// @dev Identifies a request for updating parameters.
        UpdateParams
    }

    /**
     * @dev Address constant for transaction fees, derived from the keccak256 hash of the string "txFees".
     * This address is used to store or reference the transaction fees within the contract.
     */
    address private constant TRANSACTION_FEES = address(uint160(uint256(keccak256("txFees"))));

    /**
     * @notice Constant representing a filled slot in big endian format
     * @dev Hex value 0x05 padded with leading zeros to fill 32 bytes
     */
    bytes32 constant FILLED_SLOT_BIG_ENDIAN_BYTES =
        hex"0000000000000000000000000000000000000000000000000000000000000005";

    /**
     * @dev Private variable to store the nonce value.
     * This nonce is used to ensure the uniqueness of orders.
     */
    uint256 private _nonce;

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
    mapping(bytes32 => mapping(address => uint256)) private _orders;

    /**
     * @dev Mapping to store the addresses associated with filled intents.
     * The key is a bytes32 hash representing the intent, and the value is the address
     * that filled the intent.
     */
    mapping(bytes32 => address) private _filled;

    /**
     * @dev Mapping to store instances of contracts.
     * The key the keccak(stateMachineId) and the value is the address of a known contract instance.
     */
    mapping(bytes32 => bytes32) private _instances;

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
        bytes sourceChain,
        /// @dev The state machine identifier of the destination chain
        bytes destChain,
        /// @dev The block number by which the order must be filled on the destination chain
        uint256 deadline,
        /// @dev The nonce of the order
        uint256 nonce,
        /// @dev Represents the dispatch fees associated with the IntentGateway.
        uint256 fees,
        /// @dev The tokens that the filler will provide.
        PaymentInfo[] outputs,
        /// @dev The tokens that are escrowed for the filler.
        TokenInfo[] inputs,
        /// @dev A bytes array to store the calls if any.
        bytes callData
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
    event NewDeploymentAdded(bytes stateMachineId, bytes32 gateway);

    constructor(address admin) {
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
     * @dev Fetch the IntentGateway contract instance for a chain.
     */
    function instance(bytes calldata stateMachineId) public view returns (bytes32) {
        bytes32 gateway = _instances[keccak256(stateMachineId)];
        return gateway == bytes32(0) ? bytes32(uint256(uint160(address(this)))) : gateway;
    }

    /**
     * @dev Checks that the request originates from a known instance of the IntentGateway.
     */
    modifier authenticate(PostRequest calldata request) {
        bytes32 module = request.from.length == 20
            ? bytes32(uint256(uint160(bytes20(request.from))))
            : bytes32(request.from[:32]);
        // IntentGateway only accepts incoming assets from itself or known instances
        if (instance(request.source) != module) revert Unauthorized();
        _;
    }

    /**
     * @notice Sets the parameters for the IntentGateway.
     * @param p The parameters to be set, encapsulated in a Params struct.
     */
    function setParams(Params memory p) public {
        if (msg.sender != _admin) revert Unauthorized();
        // infinite approval to save on gas
        IERC20(IDispatcher(p.host).feeToken()).approve(p.host, type(uint256).max);

        _admin = address(0);
        _params = p;
    }

    /**
     * @notice Retrieves the current parameter settings for the IntentGateway module
     * @dev Returns a struct containing all configurable parameters
     * @return Params A struct containing the module's current parameters
     */
    function params() public view returns (Params memory) {
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
     */
    function placeOrder(Order memory order, bytes32 graffiti) public payable {
        address hostAddr = host();
        // fill out the order preludes
        order.nonce = _nonce;
        order.user = bytes32(uint256(uint160(msg.sender)));
        order.sourceChain = IDispatcher(hostAddr).host();

        bytes32 commitment = keccak256(abi.encode(order));

        // escrow tokens
        uint256 msgValue = msg.value;
        uint256 inputsLen = order.inputs.length;
        for (uint256 i = 0; i < inputsLen; i++) {
            if (order.inputs[i].amount == 0) revert InvalidInput();
            address token = address(uint160(uint256(order.inputs[i].token)));
            if (token == address(0)) {
                // native token
                if (msgValue < order.inputs[i].amount) revert InsufficientNativeToken();
                msgValue -= order.inputs[i].amount;
            } else {
                IERC20(token).safeTransferFrom(msg.sender, address(this), order.inputs[i].amount);
            }

            // commit order
            _orders[commitment][token] += order.inputs[i].amount;
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
                IUniswapV2Router02(uniswapV2).swapExactETHForTokens{value: msgValue}(
                    order.fees,
                    path,
                    address(this),
                    block.timestamp
                );
            } else {
                IERC20(feeToken).safeTransferFrom(msg.sender, address(this), order.fees);
            }
            _orders[commitment][TRANSACTION_FEES] = order.fees;
        }

        _nonce += 1;
        emit OrderPlaced({
            user: order.user,
            sourceChain: order.sourceChain,
            destChain: order.destChain,
            deadline: order.deadline,
            nonce: order.nonce,
            fees: order.fees,
            outputs: order.outputs,
            inputs: order.inputs,
            callData: order.callData
        });
    }

    /**
     * @notice Fills an order with the specified options.
     * @param order The order to be filled.
     * @param options The options to be used when filling the order.
     * @dev This function is payable and can accept Ether.
     */
    function fillOrder(Order calldata order, FillOptions memory options) public payable {
        address hostAddr = host();
        // Ensure the order is being filled on the correct chain
        if (keccak256(order.destChain) != keccak256(IDispatcher(hostAddr).host())) revert WrongChain();

        // Ensure the order has not expired
        if (order.deadline < block.number) revert Expired();

        // Ensure the order has not been filled
        bytes32 commitment = keccak256(abi.encode(order));
        if (_filled[commitment] != address(0)) revert Filled();

        // fill the order
        uint256 msgValue = msg.value;
        uint256 outputsLen = order.outputs.length;
        for (uint256 i = 0; i < outputsLen; i++) {
            address token = address(uint160(uint256(order.outputs[i].token)));
            address beneficiary = address(uint160(uint256(order.outputs[i].beneficiary)));

            if (token == address(0)) {
                // native token
                if (msgValue < order.outputs[i].amount) revert InsufficientNativeToken();
                (bool sent, ) = beneficiary.call{value: order.outputs[i].amount}("");
                if (!sent) revert InsufficientNativeToken();
                msgValue -= order.outputs[i].amount;
            } else {
                IERC20(token).safeTransferFrom(msg.sender, beneficiary, order.outputs[i].amount);
            }
        }

        // dispatch calls if any
        if (order.callData.length > 0) {
            ICallDispatcher(_params.dispatcher).dispatch(order.callData);
        }

        // construct settlement message
        bytes memory data = abi.encode(
            RequestBody({
                commitment: commitment,
                tokens: order.inputs,
                beneficiary: bytes32(uint256(uint160(msg.sender)))
            })
        );
        DispatchPost memory request = DispatchPost({
            dest: order.sourceChain,
            to: abi.encodePacked(address(uint160(uint256(instance(order.sourceChain))))),
            body: bytes.concat(bytes1(uint8(RequestKind.RedeemEscrow)), data),
            timeout: 0,
            fee: options.relayerFee,
            payer: msg.sender
        });

        // dispatch settlement message
        if (msgValue > 0) {
            // there's some native tokens left to pay for request dispatch
            IDispatcher(hostAddr).dispatch{value: msgValue}(request);
        } else {
            // try to pay for dispatch with fee token
            address feeToken = IDispatcher(hostAddr).feeToken();
            uint256 fee = quote(request);
            IERC20(feeToken).safeTransferFrom(msg.sender, address(this), fee);
            IDispatcher(hostAddr).dispatch(request);
        }

        _filled[commitment] = msg.sender;
        emit OrderFilled({commitment: commitment, filler: msg.sender});
    }

    /**
     * @notice Executes an incoming post request.
     * @dev This function is called when an incoming post request is accepted.
     * It is only accessible by the host.
     * @param incoming The incoming post request data.
     */
    function onAccept(IncomingPostRequest calldata incoming) external override onlyHost {
        RequestKind kind = RequestKind(uint8(incoming.request.body[0]));
        if (kind == RequestKind.RedeemEscrow) return redeem(incoming);

        // only hyperbridge is permitted to perfom these actions
        if (keccak256(incoming.request.source) != keccak256(IDispatcher(host()).hyperbridge())) revert Unauthorized();
        if (kind == RequestKind.NewDeployment) {
            NewDeployment memory body = abi.decode(incoming.request.body[1:], (NewDeployment));
            _instances[keccak256(body.stateMachineId)] = body.gateway;

            emit NewDeploymentAdded({stateMachineId: body.stateMachineId, gateway: body.gateway});
        } else if (kind == RequestKind.UpdateParams) {
            Params memory body = abi.decode(incoming.request.body[1:], (Params));
            emit ParamsUpdated({previous: _params, current: body});

            _params = body;
        }
    }

    /**
     * @notice Redeems the escrowed tokens for an incoming post request.
     * @dev This function is marked as internal and requires authentication.
     * @param incoming The incoming post request data.
     */
    function redeem(IncomingPostRequest calldata incoming) internal authenticate(incoming.request) {
        RequestBody memory body = abi.decode(incoming.request.body[1:], (RequestBody));
        address beneficiary = address(uint160(uint256(body.beneficiary)));

        // redeem escrowed tokens
        uint256 len = body.tokens.length;
        for (uint256 i = 0; i < len; i++) {
            address token = address(uint160(uint256(body.tokens[i].token)));
            uint256 amount = body.tokens[i].amount;
            if (_orders[body.commitment][token] == 0) revert UnknownOrder();

            if (token == address(0)) {
                (bool sent, ) = beneficiary.call{value: amount}("");
                if (!sent) revert InsufficientNativeToken();
            } else {
                IERC20(token).safeTransfer(beneficiary, amount);
            }

            _orders[body.commitment][token] -= amount;
        }

        // redeem tx fees
        uint256 fees = _orders[body.commitment][TRANSACTION_FEES];
        if (fees > 0) {
            IERC20(IDispatcher(host()).feeToken()).safeTransfer(beneficiary, fees);

            delete _orders[body.commitment][TRANSACTION_FEES];
        }

        _filled[body.commitment] = incoming.relayer;

        emit EscrowReleased({commitment: body.commitment});
    }

    /**
     * @notice Cancels an existing order.
     * @param order The order to be canceled.
     * @param options Additional options for the cancellation process.
     * @dev This function can only be called by the order owner and requires a payment.
     * It will initiate a storage query on the source chain to verify the order has not
     * yet been filled. If the order has not been filled, the tokens will be released.
     */
    function cancelOrder(Order calldata order, CancelOptions memory options) public payable {
        bytes32 commitment = keccak256(abi.encode(order));

        // only owner can cancel order
        if (order.user != bytes32(uint256(uint160(msg.sender)))) revert Unauthorized();

        // order has not yet expired
        if (options.height <= order.deadline) revert NotExpired();

        // order has already been filled
        if (_filled[commitment] != address(0)) revert Filled();

        // fetch the tokens
        uint256 inputsLen = order.inputs.length;
        for (uint256 i = 0; i < inputsLen; i++) {
            // check for order existence
            if (_orders[commitment][address(uint160(uint256(order.inputs[i].token)))] == 0) revert UnknownOrder();
        }

        bytes memory context = abi.encode(
            RequestBody({commitment: commitment, tokens: order.inputs, beneficiary: order.user})
        );

        bytes[] memory keys = new bytes[](1);
        keys[0] = bytes.concat(
            // contract address
            abi.encodePacked(address(uint160(uint256(instance(order.destChain))))),
            // storage slot hash
            calculateCommitmentSlotHash(commitment)
        );
        DispatchGet memory request = DispatchGet({
            dest: order.destChain,
            keys: keys,
            timeout: 0,
            height: uint64(options.height),
            fee: options.relayerFee,
            context: context
        });

        // dispatch storage query request
        address hostAddr = host();
        if (msg.value > 0) {
            // there's some native tokens left to pay for request dispatch
            IDispatcher(hostAddr).dispatch{value: msg.value}(request);
        } else {
            // try to pay for dispatch with fee token
            address feeToken = IDispatcher(hostAddr).feeToken();
            uint256 fee = quote(request);
            IERC20(feeToken).safeTransferFrom(msg.sender, address(this), fee);
            IDispatcher(hostAddr).dispatch(request);
        }
    }

    /**
     * @notice Handles the response for an incoming GET request.
     * @dev This function is called by the host to process the response of a GET request.
     * @param incoming The response data structure for the GET request.
     * Only the host can call this function.
     */
    function onGetResponse(IncomingGetResponse memory incoming) external override onlyHost {
        if (incoming.response.values[0].value.length != 0) revert Filled();

        RequestBody memory body = abi.decode(incoming.response.request.context, (RequestBody));
        address beneficiary = address(uint160(uint256(body.beneficiary)));

        // recover escrowed tokens
        uint256 len = body.tokens.length;
        for (uint256 i = 0; i < len; i++) {
            address token = address(uint160(uint256(body.tokens[i].token)));
            uint256 amount = body.tokens[i].amount;
            if (_orders[body.commitment][token] == 0) revert UnknownOrder();

            if (token == address(0)) {
                (bool sent, ) = beneficiary.call{value: amount}("");
                if (!sent) revert InsufficientNativeToken();
            } else {
                IERC20(token).safeTransfer(beneficiary, amount);
            }

            _orders[body.commitment][token] -= amount;
        }

        // recover tx fees
        uint256 fees = _orders[body.commitment][TRANSACTION_FEES];
        if (fees > 0) {
            IERC20(IDispatcher(host()).feeToken()).safeTransfer(beneficiary, fees);

            delete _orders[body.commitment][TRANSACTION_FEES];
        }

        emit EscrowRefunded({commitment: body.commitment});
    }
}
