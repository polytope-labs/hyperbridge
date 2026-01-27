// SPDX-License-Identifier: UNLICENSED
// A Sample ISMP solidity contract for unit tests

pragma solidity ^0.8.17;

import {IncomingPostRequest, IncomingPostResponse, IncomingGetResponse} from "@hyperbridge/core/interfaces/IApp.sol";
import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";
import {Message, PostRequest, PostResponse, GetRequest} from "@hyperbridge/core/libraries/Message.sol";
import {
    IDispatcher,
    DispatchPost,
    DispatchPostResponse,
    DispatchGet
} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {StorageValue} from "@polytope-labs/solidity-merkle-trees/src/Types.sol";

struct PingMessage {
    bytes dest;
    address module;
    uint64 timeout;
    uint256 count;
    uint256 fee;
}

interface ITokenFaucet {
    // drips the feeToken once per day
    function drip(address) external;
}

contract PingModule is HyperApp {
    using Message for PostResponse;
    using Message for PostRequest;
    using Message for GetRequest;

    event PostResponseReceived();
    event GetResponseReceived(StorageValue[] message);
    event PostRequestTimeoutReceived();
    event PostResponseTimeoutReceived();
    event GetTimeoutReceived();
    event PostReceived(string message);
    event MessageDispatched();

    error NotIsmpHost();
    error ExecutionFailed();

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

    function setIsmpHost(address hostAddr, address tokenFaucet) public onlyAdmin {
        address feeToken = IHost(hostAddr).feeToken();
        IERC20(feeToken).approve(hostAddr, type(uint256).max);
        if (tokenFaucet != address(0)) {
            ITokenFaucet(tokenFaucet).drip(feeToken);
        }

        _host = hostAddr;
    }

    function previousPostRequest() public view returns (PostRequest memory) {
        return _request;
    }

    // returns the current ismp host set
    function host() public view override returns (address) {
        return _host;
    }

    function dispatchPostResponse(PostResponse memory response) public returns (bytes32) {
        uint256 perByteFee = IHost(_host).perByteFee(response.request.source);
        address feeToken = IHost(_host).feeToken();
        uint256 length = 32 > response.response.length ? 32 : response.response.length;
        uint256 fee = perByteFee * length;

        IERC20(feeToken).transferFrom(msg.sender, address(this), fee);
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
        uint256 perByteFee = IHost(_host).perByteFee(request.dest);
        address feeToken = IHost(_host).feeToken();
        uint256 length = 32 > request.body.length ? 32 : request.body.length;
        uint256 fee = perByteFee * length;

        IERC20(feeToken).transferFrom(msg.sender, address(this), fee);
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
            context: new bytes(0),
            fee: 0
        });

        return IDispatcher(_host).dispatch(get);
    }

    function ping(PingMessage memory pingMessage) public {
        bytes memory body = bytes.concat("hello from ", IHost(_host).host());
        uint256 perByteFee = IHost(_host).perByteFee(pingMessage.dest);
        address feeToken = IHost(_host).feeToken();
        uint256 length = 32 > body.length ? 32 : body.length;
        uint256 fee = (pingMessage.fee + (perByteFee * length)) * pingMessage.count;

        IERC20(feeToken).transferFrom(msg.sender, address(this), fee);

        for (uint256 i = 0; i < pingMessage.count; i++) {
            DispatchPost memory post = DispatchPost({
                body: bytes.concat("hello from ", IHost(_host).host()),
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

    function onAccept(IncomingPostRequest memory incoming) external override onlyHost {
        emit PostReceived(string(incoming.request.body));
        _request = incoming.request;
    }

    function onGetResponse(IncomingGetResponse memory incoming) external override onlyHost {
        emit GetResponseReceived(incoming.response.values);
    }

    function onPostResponse(IncomingPostResponse memory) external override onlyHost {
        emit PostResponseReceived();
    }

    function onGetTimeout(GetRequest memory) external override onlyHost {
        emit GetTimeoutReceived();
    }

    function onPostRequestTimeout(PostRequest memory) external override onlyHost {
        emit PostRequestTimeoutReceived();
    }

    function onPostResponseTimeout(PostResponse memory) external override onlyHost {
        emit PostResponseTimeoutReceived();
    }
}
