// SPDX-License-Identifier: UNLICENSED
// Minimal HyperApp used by integration tests as a stand-in dispatcher.
// Not deployed in production; lives under tests/ so forge builds it but it ships nowhere.

pragma solidity ^0.8.17;

import {HyperApp} from "@hyperbridge/core/apps/HyperApp.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";
import {PostRequest, GetRequest} from "@hyperbridge/core/libraries/Message.sol";
import {
    IDispatcher,
    DispatchPost,
    DispatchGet
} from "@hyperbridge/core/interfaces/IDispatcher.sol";
import {
    IncomingPostRequest,
    IncomingGetResponse,
    PostRequestTimeout,
    GetRequestTimeout
} from "@hyperbridge/core/interfaces/IApp.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract TestDispatcher is HyperApp {
    address internal _host;

    constructor(address) {}

    function setIsmpHost(address hostAddr, address /*tokenFaucet*/) external {
        address feeToken = IHost(hostAddr).feeToken();
        IERC20(feeToken).approve(hostAddr, type(uint256).max);
        _host = hostAddr;
    }

    function host() public view override returns (address) {
        return _host;
    }

    function dispatchPostRequest(PostRequest memory request) external returns (bytes32) {
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

    function dispatchGetRequest(GetRequest memory request) external returns (bytes32) {
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

    function onAccept(IncomingPostRequest calldata) external override onlyHost {}
    function onPostRequestTimeout(PostRequestTimeout memory) external override onlyHost {}
    function onGetResponse(IncomingGetResponse memory) external override onlyHost {}
    function onGetTimeout(GetRequestTimeout memory) external override onlyHost {}
}
