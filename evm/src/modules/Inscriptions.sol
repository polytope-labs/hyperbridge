// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity ^0.8.17;

import {BaseIsmpModule} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {DispatchPost, IDispatcher} from "@polytope-labs/ismp-solidity/IDispatcher.sol";

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

    address private _admin;
    address private _host;

    constructor(address admin) {
        _admin = admin;
    }

    function setHost(address host) public {
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

        if (msg.value > 0) {
            // there's some native tokens left to pay for request dispatch
            IDispatcher(_host).dispatch{value: msg.value}(request);
        } else {
            // try to pay for dispatch with fee token
            uint256 fee = (IDispatcher(_host).perByteFee() * params.message.length) + params.relayerFee;
            IERC20(IDispatcher(_host).feeToken()).safeTransferFrom(msg.sender, address(this), fee);
            IDispatcher(_host).dispatch(request);
        }
    }
}
