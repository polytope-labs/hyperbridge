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

import {PostRequest, PostResponse, GetRequest} from "../libraries/Message.sol";
import {DispatchPost, DispatchPostResponse, DispatchGet, IDispatcher} from "../interfaces/IDispatcher.sol";
import {IApp, IncomingPostRequest, IncomingPostResponse, IncomingGetResponse} from "../interfaces/IApp.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @dev Uniswap interface for estimating fees in the native token
 */
interface IUniswapV2Router02 {
    function WETH() external pure returns (address);

    function getAmountsIn(uint256, address[] calldata) external pure returns (uint256[] memory);
}

/**
 * @title HyperApp
 * @notice Abstract base contract for building cross-chain applications on Hyperbridge
 * @dev Provides a simplified interface for implementing `IApp` with built-in utilities for fee estimation,
 * host authorization, and cross-chain message handling. Extend this contract to build your Hyperbridge application.
 */
abstract contract HyperApp is IApp {
    using SafeERC20 for IERC20;

    /**
     * @dev Call was not expected
     */
    error UnexpectedCall();

    /**
     * @dev Account is unauthorized
     */
    error UnauthorizedCall();

    /**
     * @dev restricts caller to the local `Host`
     */
    modifier onlyHost() {
        if (msg.sender != host()) revert UnauthorizedCall();
        _;
    }

    /**
     * @dev Should return the `Host` address for the current chain.
     * The `Host` is an immutable contract that will never change.
     */
    function host() public view virtual returns (address);

    /**
     * @dev returns the quoted fee in the feeToken for dispatching a POST request
     */
    function quote(DispatchPost memory request) public view returns (uint256) {
        uint256 len = 32 > request.body.length ? 32 : request.body.length;
        return request.fee + (len * IDispatcher(host()).perByteFee(request.dest));
    }

    /**
     * @dev returns the quoted fee in the feeToken for dispatching a GET request
     */
    function quote(DispatchGet memory request) public view returns (uint256) {
        address _host = host();
        uint256 pbf = IDispatcher(_host).perByteFee(IDispatcher(_host).host());
        uint256 minimumFee = 32 * pbf;
        uint256 totalFee = request.fee + (pbf * request.context.length);
        return minimumFee > totalFee ? minimumFee : totalFee;
    }

    /**
     * @dev returns the quoted fee in the feeToken for dispatching a POST response
     */
    function quote(DispatchPostResponse memory response) public view returns (uint256) {
        uint256 len = 32 > response.response.length ? 32 : response.response.length;
        return response.fee + (len * IDispatcher(host()).perByteFee(response.request.source));
    }

    /**
     * @dev returns the quoted fee in the native token for dispatching a POST request
     */
    function quoteNative(DispatchPost memory request) public view returns (uint256) {
        uint256 fee = quote(request);
        address _host = host();
        address _uniswap = IDispatcher(_host).uniswapV2Router();
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(_uniswap).WETH();
        path[1] = IDispatcher(_host).feeToken();
        return IUniswapV2Router02(_uniswap).getAmountsIn(fee, path)[0];
    }

    /**
     * @dev returns the quoted fee in the native token for dispatching a GET request
     */
    function quoteNative(DispatchGet memory request) public view returns (uint256) {
        uint256 fee = quote(request);
        address _host = host();
        address _uniswap = IDispatcher(_host).uniswapV2Router();
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(_uniswap).WETH();
        path[1] = IDispatcher(_host).feeToken();
        return IUniswapV2Router02(_uniswap).getAmountsIn(fee, path)[0];
    }

    /**
     * @dev returns the quoted fee in the native token for dispatching a POST response
     */
    function quoteNative(DispatchPostResponse memory request) public view returns (uint256) {
        uint256 fee = quote(request);
        address _host = host();
        address _uniswap = IDispatcher(_host).uniswapV2Router();
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(_uniswap).WETH();
        path[1] = IDispatcher(_host).feeToken();
        return IUniswapV2Router02(_uniswap).getAmountsIn(fee, path)[0];
    }

    /**
     * @notice Dispatches a POST request using the fee token for payment
     * @dev Handles fee token approval and transfer before dispatching the request to the Host.
     * If the payer is not this contract, transfers fee tokens from the payer to this contract first.
     * @param request The POST request to dispatch containing destination, body, timeout, and fee parameters
     * @param payer The address that will pay the fee token. If different from this contract, must have approved this contract to spend the fee amount
     * @return commitment The unique identifier for the dispatched request
     */
    function dispatchWithFeeToken(DispatchPost memory request, address payer) internal returns (bytes32) {
        address hostAddr = host();
        address feeToken = IDispatcher(hostAddr).feeToken();
        uint256 fee = quote(request);
        if (payer != address(this)) IERC20(feeToken).safeTransferFrom(payer, address(this), fee);
        IERC20(feeToken).forceApprove(hostAddr, fee);
        return IDispatcher(hostAddr).dispatch(request);
    }

    /**
     * @notice Dispatches a POST response using the fee token for payment
     * @dev Handles fee token approval and transfer before dispatching the response to the Host.
     * If the payer is not this contract, transfers fee tokens from the payer to this contract first.
     * @param response The POST response to dispatch containing the original request, response data, timeout, and fee parameters
     * @param payer The address that will pay the fee token. If different from this contract, must have approved this contract to spend the fee amount
     * @return commitment The unique identifier for the dispatched response
     */
    function dispatchWithFeeToken(DispatchPostResponse memory response, address payer) internal returns (bytes32) {
        address hostAddr = host();
        address feeToken = IDispatcher(hostAddr).feeToken();
        uint256 fee = quote(response);
        if (payer != address(this)) IERC20(feeToken).safeTransferFrom(payer, address(this), fee);
        IERC20(feeToken).forceApprove(hostAddr, fee);
        return IDispatcher(hostAddr).dispatch(response);
    }

    /**
     * @notice Dispatches a GET request using the fee token for payment
     * @dev Handles fee token approval and transfer before dispatching the request to the Host.
     * If the payer is not this contract, transfers fee tokens from the payer to this contract first.
     * @param request The GET request to dispatch containing destination, keys, height, timeout, and fee parameters
     * @param payer The address that will pay the fee token. If different from this contract, must have approved this contract to spend the fee amount
     * @return commitment The unique identifier for the dispatched request
     */
    function dispatchWithFeeToken(DispatchGet memory request, address payer) internal returns (bytes32) {
        address hostAddr = host();
        address feeToken = IDispatcher(hostAddr).feeToken();
        uint256 fee = quote(request);
        if (payer != address(this)) IERC20(feeToken).safeTransferFrom(payer, address(this), fee);
        IERC20(feeToken).forceApprove(hostAddr, fee);
        return IDispatcher(hostAddr).dispatch(request);
    }

    /**
     * @notice Callback for receiving incoming POST requests
     * @dev Called by the Host when a new POST request is received for this app.
     * Override this method to implement request handling logic. The default implementation reverts.
     */
    function onAccept(IncomingPostRequest calldata) external virtual onlyHost {
        revert UnexpectedCall();
    }

    /**
     * @notice Callback for handling POST request timeouts
     * @dev Called by the Host when a previously dispatched POST request has timed out.
     * Override this method to implement cleanup or retry logic. The default implementation reverts.
     */
    function onPostRequestTimeout(PostRequest memory) external virtual onlyHost {
        revert UnexpectedCall();
    }

    /**
     * @notice Callback for receiving POST responses
     * @dev Called by the Host when a response is received for a previously dispatched POST request.
     * Override this method to process response data. The default implementation reverts.
     */
    function onPostResponse(IncomingPostResponse memory) external virtual onlyHost {
        revert UnexpectedCall();
    }

    /**
     * @notice Callback for handling POST response timeouts
     * @dev Called by the Host when a POST response that was sent has timed out.
     * Override this method to handle response timeout scenarios. The default implementation reverts.
     */
    function onPostResponseTimeout(PostResponse memory) external virtual onlyHost {
        revert UnexpectedCall();
    }

    /**
     * @notice Callback for receiving GET responses
     * @dev Called by the Host when a response is received for a previously dispatched GET request.
     * Override this method to process the retrieved state data. The default implementation reverts.
     */
    function onGetResponse(IncomingGetResponse memory) external virtual onlyHost {
        revert UnexpectedCall();
    }

    /**
     * @notice Callback for handling GET request timeouts
     * @dev Called by the Host when a previously dispatched GET request has timed out.
     * Override this method to handle GET timeout scenarios. The default implementation reverts.
     */
    function onGetTimeout(GetRequest memory) external virtual onlyHost {
        revert UnexpectedCall();
    }
}
