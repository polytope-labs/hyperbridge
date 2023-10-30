// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity 0.8.17;

import "ismp/IIsmpModule.sol";
import "ismp/IIsmpHost.sol";
import "ismp/StateMachine.sol";

struct PingMessage {
    bytes dest;
    address module;
    uint64 timeout;
}

contract PingModule is IIsmpModule {
    event PostResponseReceived();
    event GetResponseReceived();
    event PostTimeoutReceived();
    event GetTimeoutReceived();
    event PostReceived(string message);
    event MessageDispatched();

    error NotIsmpHost();
    error ExecutionFailed();

    // restricts call to `IIsmpHost`
    modifier onlyIsmpHost() {
        if (msg.sender != _host) {
            revert NotIsmpHost();
        }
        _;
    }

    address internal _host;

    constructor(address host) {
        _host = host;
    }

    function dispatch(PostRequest memory request) public returns (bytes32) {
        bytes32 commitment = Message.hash(request);
        DispatchPost memory post = DispatchPost({
            body: request.body,
            dest: request.dest,
            timeout: request.timeoutTimestamp,
            to: request.to,
            gaslimit: request.gaslimit
        });
        IIsmp(_host).dispatch(post);
        return commitment;
    }

    function dispatch(GetRequest memory request) public returns (bytes32) {
        bytes32 commitment = Message.hash(request);
        DispatchGet memory get = DispatchGet({
            dest: request.dest,
            height: request.height,
            keys: request.keys,
            timeout: request.timeoutTimestamp,
            gaslimit: request.gaslimit
        });
        IIsmp(_host).dispatch(get);
        return commitment;
    }

    function ping(PingMessage memory msg) public {
        DispatchPost memory post = DispatchPost({
            body: bytes.concat("hello from ", IIsmpHost(_host).host()),
            dest: msg.dest,
            // one hour
            timeout: msg.timeout,
            // instance of this pallet on another chain.
            to: abi.encodePacked(address(msg.module)),
            // unused for now
            gaslimit: 0
        });
        IIsmp(_host).dispatch(post);
    }

    function dispatchToParachain(uint256 _paraId) public {
        DispatchPost memory post = DispatchPost({
            body: bytes("hello from ethereum"),
            dest: StateMachine.kusama(_paraId),
            timeout: 0,
            // timeout: 60 * 60, // one hour
            to: bytes("ismp-ast"), // ismp demo pallet
            gaslimit: 0 // unnedeed, since it's a pallet
        });
        IIsmp(_host).dispatch(post);
    }

    function onAccept(PostRequest memory request) external onlyIsmpHost {
        emit PostReceived(string(request.body));
    }

    function onPostResponse(PostResponse memory response) external onlyIsmpHost {
        emit PostResponseReceived();
    }

    function onGetResponse(GetResponse memory response) external onlyIsmpHost {
        emit GetResponseReceived();
    }

    function onGetTimeout(GetRequest memory request) external onlyIsmpHost {
        emit GetTimeoutReceived();
    }

    function onPostTimeout(PostRequest memory request) external onlyIsmpHost {
        emit PostTimeoutReceived();
    }
}
