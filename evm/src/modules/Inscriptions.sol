// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity ^0.8.17;

import {BaseIsmpModule, IncomingPostRequest} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {DispatchPost, IDispatcher, PostRequest} from "@polytope-labs/ismp-solidity/IDispatcher.sol";

struct CrossChainMessage {
    bytes dest;
    bytes message;
    uint256 relayerFee;
    uint64 timeout;
}

contract CrossChainInscription is BaseIsmpModule {
    using SafeERC20 for IERC20;

    // An inscription has been received
    event PostReceived(string message);

    // An inscription has been received
    event PostDispatched(string message, bytes32 commitment);

    // An inscription has been received
    event PostTimedOut(string message);

    // Call is unauthorized
    error Unauthorized();

    address private _admin;
    address private _host;

    constructor(address admin) {
        _admin = admin;
    }

    function setHost(address host) public {
        if (msg.sender != _admin) revert Unauthorized();
        // infinite approval to save on gas
        IERC20(IDispatcher(host).feeToken()).approve(
            host,
            type(uint256).max
        );

        _host = host;
        _admin = address(0);
    }

    function inscribe(CrossChainMessage memory params) public payable {
        DispatchPost memory request = DispatchPost({
            body: params.message,
            dest: params.dest,
            timeout: params.timeout,
            // instance of this pallet on another chain.
            to: abi.encodePacked(address(this)),
            fee: params.relayerFee,
            payer: address(this)
        });

        bytes32 commitment;
        if (msg.value > 0) {
            // there's some native tokens left to pay for request dispatch
            commitment = IDispatcher(_host).dispatch{value: msg.value}(request);
        } else {
            // try to pay for dispatch with fee token
            uint256 length = 32 > params.message.length ? 32 : params.message.length;
            uint256 fee = (IDispatcher(_host).perByteFee(params.dest) * length) + params.relayerFee;
            IERC20(IDispatcher(_host).feeToken()).safeTransferFrom(msg.sender, address(this), fee);
            commitment = IDispatcher(_host).dispatch(request);
        }

        emit PostDispatched(string(params.message), commitment);
    }

    function onAccept(IncomingPostRequest memory incoming) external override {
        if (msg.sender != _host) revert UnauthorizedCall();
        emit PostReceived(string(incoming.request.body));
    }

    function onPostRequestTimeout(PostRequest calldata request) external override {
        if (msg.sender != _host) revert UnauthorizedCall();
        emit PostTimedOut(string(request.body));
    }
}
