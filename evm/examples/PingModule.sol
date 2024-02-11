// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity 0.8.17;

import "ismp/IIsmpModule.sol";
import "ismp/IIsmpHost.sol";
import "ismp/StateMachine.sol";
import "ismp/IIsmp.sol";

struct PingMessage {
    bytes dest;
    address module;
    uint64 timeout;
    uint256 count;
    uint256 fee;
}

contract PingModule is IIsmpModule {
    using Message for PostResponse;
    using Message for PostRequest;
    using Message for GetRequest;

    event PostResponseReceived();
    event GetResponseReceived();
    event PostRequestTimeoutReceived();
    event PostResponseTimeoutReceived();
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

    // restricts call to `admin`
    modifier onlyAdmin() {
        if (msg.sender != _admin) {
            revert NotIsmpHost();
        }
        _;
    }

    address internal _host;
    address internal _admin;

    constructor(address admin) {
        _admin = admin;
    }

    function setIsmpHost(address hostAddr) public onlyAdmin {
        _host = hostAddr;
    }

    // returns the current ismp host set
    function host() public view returns (address) {
        return _host;
    }

    function dispatchPostResponse(PostResponse memory response) public returns (bytes32) {
        DispatchPostResponse memory post = DispatchPostResponse({
            request: response.request,
            response: response.response,
            timeout: response.timeoutTimestamp,
            gaslimit: response.gaslimit,
            fee: 0
        });
        IIsmp(_host).dispatch(post);
        return response.hash();
    }

    function dispatch(PostRequest memory request) public returns (bytes32) {
        DispatchPost memory post = DispatchPost({
            body: request.body,
            dest: request.dest,
            timeout: request.timeoutTimestamp,
            to: request.to,
            gaslimit: request.gaslimit,
            fee: 0
        });
        IIsmp(_host).dispatch(post);
        return request.hash();
    }

    function dispatch(GetRequest memory request) public returns (bytes32) {
        DispatchGet memory get = DispatchGet({
            dest: request.dest,
            height: request.height,
            keys: request.keys,
            timeout: request.timeoutTimestamp,
            gaslimit: request.gaslimit,
            fee: 0
        });
        IIsmp(_host).dispatch(get);
        return request.hash();
    }

    function ping(PingMessage memory pingMessage) public {
        for (uint256 i = 0; i < pingMessage.count; i++) {
            DispatchPost memory post = DispatchPost({
                body: bytes.concat("hello from ", IIsmpHost(_host).host()),
                dest: pingMessage.dest,
                // one hour
                timeout: pingMessage.timeout,
                // instance of this pallet on another chain.
                to: abi.encodePacked(address(pingMessage.module)),
                // unused for now
                gaslimit: 0,
                fee: pingMessage.fee
            });
            IIsmp(_host).dispatch(post);
        }
    }

    function dispatchToParachain(uint256 _paraId) public {
        DispatchPost memory post = DispatchPost({
            body: bytes("hello from evm"),
            dest: StateMachine.kusama(_paraId),
            timeout: 0,
            // timeout: 60 * 60, // one hour
            to: bytes("ismp-ast"), // ismp demo pallet
            gaslimit: 0,
            fee: 0
        });
        IIsmp(_host).dispatch(post);
    }

    function onAccept(PostRequest memory request) external onlyIsmpHost {
        emit PostReceived(string(request.body));
    }

    function onPostResponse(PostResponse memory) external onlyIsmpHost {
        emit PostResponseReceived();
    }

    function onGetResponse(GetResponse memory) external onlyIsmpHost {
        emit GetResponseReceived();
    }

    function onGetTimeout(GetRequest memory) external onlyIsmpHost {
        emit GetTimeoutReceived();
    }

    function onPostRequestTimeout(PostRequest memory) external onlyIsmpHost {
        emit PostRequestTimeoutReceived();
    }

    function onPostResponseTimeout(PostResponse memory) external onlyIsmpHost {
        emit PostResponseTimeoutReceived();
    }
}
