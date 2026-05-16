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

import {PostRequest, GetRequest} from "../libraries/Message.sol";
import {DispatchPost, DispatchGet, IDispatcher} from "../interfaces/IDispatcher.sol";
import {IApp, IncomingPostRequest, IncomingGetResponse} from "../interfaces/IApp.sol";

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
     * @dev returns the quoted fee in the native token for dispatching a POST request
     */
    function quote(DispatchPost memory request) public view returns (uint256) {
        address _host = host();
        address _uniswap = IDispatcher(_host).uniswapV2Router();
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(_uniswap).WETH();
        path[1] = IDispatcher(_host).feeToken();
        return IUniswapV2Router02(_uniswap).getAmountsIn(request.fee, path)[0];
    }

    /**
     * @dev returns the quoted fee in the native token for dispatching a GET request
     */
    function quote(DispatchGet memory request) public view returns (uint256) {
        address _host = host();
        address _uniswap = IDispatcher(_host).uniswapV2Router();
        address[] memory path = new address[](2);
        path[0] = IUniswapV2Router02(_uniswap).WETH();
        path[1] = IDispatcher(_host).feeToken();
        return IUniswapV2Router02(_uniswap).getAmountsIn(request.fee, path)[0];
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
        if (payer != address(this)) IERC20(feeToken).safeTransferFrom(payer, address(this), request.fee);
        IERC20(feeToken).forceApprove(hostAddr, request.fee);
        return IDispatcher(hostAddr).dispatch(request);
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
        if (payer != address(this)) IERC20(feeToken).safeTransferFrom(payer, address(this), request.fee);
        IERC20(feeToken).forceApprove(hostAddr, request.fee);
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
