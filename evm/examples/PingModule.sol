// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity 0.8.17;

import "ismp/IIsmpModule.sol";
import "ismp/IIsmpHost.sol";
import "ismp/StateMachine.sol";
import "ismp/Message.sol";
import "ismp/IDispatcher.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";

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
    PostRequest private _request;

    constructor(address admin) {
        _admin = admin;
    }

    function setIsmpHost(address hostAddr) public onlyAdmin {
        _host = hostAddr;
    }

    function previousPostRequest() public view returns (PostRequest memory) {
        return _request;
    }

    // returns the current ismp host set
    function host() public view returns (address) {
        return _host;
    }

    function dispatchPostResponse(PostResponse memory response) public returns (bytes32) {
        uint256 perByteFee = IIsmpHost(_host).perByteFee();
        address feeToken = IIsmpHost(_host).feeToken();
        uint256 fee = perByteFee * response.response.length;

        IERC20(feeToken).transferFrom(msg.sender, address(this), fee);
        IERC20(feeToken).approve(_host, fee);
        DispatchPostResponse memory post = DispatchPostResponse({
            request: response.request,
            response: response.response,
            timeout: response.timeoutTimestamp,
            fee: 0,
            payer: tx.origin
        });
        return IDispatcher(_host).dispatch(post);
    }

    function dispatch(PostRequest memory request) public returns (bytes32) {
        uint256 perByteFee = IIsmpHost(_host).perByteFee();
        address feeToken = IIsmpHost(_host).feeToken();
        uint256 fee = perByteFee * request.body.length;

        IERC20(feeToken).transferFrom(msg.sender, address(this), fee);
        IERC20(feeToken).approve(_host, fee);

        DispatchPost memory post = DispatchPost({
            body: request.body,
            dest: request.dest,
            timeout: request.timeoutTimestamp,
            to: request.to,
            fee: 0,
            payer: tx.origin
        });

        return IDispatcher(_host).dispatch(post);
    }

    function dispatch(GetRequest memory request) public returns (bytes32) {
        DispatchGet memory get = DispatchGet({
            dest: request.dest,
            height: request.height,
            keys: request.keys,
            timeout: request.timeoutTimestamp,
            sender: msg.sender,
            fee: 0
        });

        return IDispatcher(_host).dispatch(get);
    }

    function ping(PingMessage memory pingMessage) public {
        bytes memory body = bytes.concat("hello from ", IIsmpHost(_host).host());
        uint256 perByteFee = IIsmpHost(_host).perByteFee();
        address feeToken = IIsmpHost(_host).feeToken();
        uint256 fee = (pingMessage.fee + (perByteFee * body.length)) * pingMessage.count;

        IERC20(feeToken).transferFrom(msg.sender, address(this), fee);
        IERC20(feeToken).approve(_host, fee);

        for (uint256 i = 0; i < pingMessage.count; i++) {
            DispatchPost memory post = DispatchPost({
                body: bytes.concat("hello from ", IIsmpHost(_host).host()),
                dest: pingMessage.dest,
                timeout: pingMessage.timeout,
                to: abi.encodePacked(address(pingMessage.module)),
                fee: pingMessage.fee,
                payer: tx.origin
            });
            IDispatcher(_host).dispatch(post);
        }
    }

    function dispatchToParachain(uint256 _paraId) public {
        DispatchPost memory post = DispatchPost({
            body: bytes("hello from evm"),
            dest: StateMachine.kusama(_paraId),
            timeout: 0,
            to: bytes("ismp-ast"), // ismp demo pallet
            fee: 0,
            payer: tx.origin
        });
        IDispatcher(_host).dispatch(post);
    }

    function onAccept(IncomingPostRequest memory incoming) external onlyIsmpHost {
        emit PostReceived(string(incoming.request.body));
        _request = incoming.request;
    }

    function onPostResponse(IncomingPostResponse memory) external onlyIsmpHost {
        emit PostResponseReceived();
    }

    function onGetResponse(IncomingGetResponse memory) external onlyIsmpHost {
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
