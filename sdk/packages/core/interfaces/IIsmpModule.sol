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

import {PostRequest, PostResponse, GetResponse, GetRequest} from "./Message.sol";
import {DispatchPost, DispatchPostResponse, DispatchGet, IDispatcher} from "./IDispatcher.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

struct IncomingPostRequest {
	// The Post request
	PostRequest request;
	// Relayer responsible for delivering the request
	address relayer;
}

struct IncomingPostResponse {
	// The Post response
	PostResponse response;
	// Relayer responsible for delivering the response
	address relayer;
}

struct IncomingGetResponse {
	// The Get response
	GetResponse response;
	// Relayer responsible for delivering the response
	address relayer;
}

interface IIsmpModule {
	/**
	 * @dev Called by the `IsmpHost` to notify a module of a new request the module may choose to respond immediately, or in a later block
	 * @param incoming post request
	 */
	function onAccept(IncomingPostRequest memory incoming) external;

	/**
	 * @dev Called by the `IsmpHost` to notify a module of a post response to a previously sent out request
	 * @param incoming post response
	 */
	function onPostResponse(IncomingPostResponse memory incoming) external;

	/**
	 * @dev Called by the `IsmpHost` to notify a module of a get response to a previously sent out request
	 * @param incoming get response
	 */
	function onGetResponse(IncomingGetResponse memory incoming) external;

	/**
	 * @dev Called by the `IsmpHost` to notify a module of post requests that were previously sent but have now timed-out
	 * @param request post request
	 */
	function onPostRequestTimeout(PostRequest memory request) external;

	/**
	 * @dev Called by the `IsmpHost` to notify a module of post requests that were previously sent but have now timed-out
	 * @param request post request
	 */
	function onPostResponseTimeout(PostResponse memory request) external;

	/**
	 * @dev Called by the `IsmpHost` to notify a module of get requests that were previously sent but have now timed-out
	 * @param request get request
	 */
	function onGetTimeout(GetRequest memory request) external;
}


/**
 * @dev Uniswap interface for estimating fees in the native token
 */
interface IUniswapV2Router02 {
	function WETH() external pure returns (address);
	function getAmountsIn(uint, address[] calldata) external pure returns (uint[] memory);
}

/**
 * @dev Abstract contract to make implementing `IIsmpModule` easier.
 */
abstract contract BaseIsmpModule is IIsmpModule {
	/**
	 * @dev Call was not expected
	 */
	error UnexpectedCall();

	/**
	 * @dev Account is unauthorized
	 */
	error UnauthorizedCall();

	/**
	 * @dev restricts caller to the local `IsmpHost`
	 */
	modifier onlyHost() {
		if (msg.sender != host()) revert UnauthorizedCall();
		_;
	}

	constructor() {
		address hostAddr = host();
		if (hostAddr != address(0)) {
			// approve the host infintely
			IERC20(IDispatcher(hostAddr).feeToken()).approve(hostAddr, type(uint256).max);
		}
	}

	/**
	 * @dev Should return the `IsmpHost` address for the current chain.
	 * The `IsmpHost` is an immutable contract that will never change.
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
	 * @dev returns the quoted fee in the feeToken for dispatching a POST response
	 */
	function quote(DispatchPostResponse memory response) public view returns (uint256) {
		uint256 len = 32 > response.response.length ? 32 : response.response.length;
		return response.fee + (len * IDispatcher(host()).perByteFee(response.request.source));
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

	function onAccept(IncomingPostRequest calldata) external virtual onlyHost {
		revert UnexpectedCall();
	}

	function onPostRequestTimeout(PostRequest memory) external virtual onlyHost {
		revert UnexpectedCall();
	}

	function onPostResponse(IncomingPostResponse memory) external virtual onlyHost {
		revert UnexpectedCall();
	}

	function onPostResponseTimeout(PostResponse memory) external virtual onlyHost {
		revert UnexpectedCall();
	}

	function onGetResponse(IncomingGetResponse memory) external virtual onlyHost {
		revert UnexpectedCall();
	}

	function onGetTimeout(GetRequest memory) external virtual onlyHost {
		revert UnexpectedCall();
	}
}
