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

import {BaseIsmpModule, PostRequest, IncomingPostRequest} from "@polytope-labs/ismp-solidity/IIsmpModule.sol";
import {IDispatcher, DispatchPost} from "@polytope-labs/ismp-solidity/IDispatcher.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Bytes} from "@polytope-labs/solidity-merkle-trees/src/trie/Bytes.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";

struct RequestBody {
    // The asset owner
    address owner;
    // The assetId to create
    bytes32 assetId;
}

struct RegistrarParams {
    // Ismp host
    address host;
    // registration base fee
    uint256 baseFee;
}

/**
 * @title The TokenRegistrar.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Serves as a form of gas abstraction for token registration.
 * By collecting fees on any chain and permitting token creation on the
 * Hyperbridge chain.
 */
contract TokenRegistrar is BaseIsmpModule {
    using Bytes for bytes;

    RegistrarParams private _params;

    // admin account
    address private _admin;

    // Unexpected state
    error InconsistentState();

    // Requested action is unauthorized
    error UnauthorizedAction();

    // @notice A user has initiated the asset registration process
    event RegistrationBegun(bytes32 indexed assetId, address indexed owner, uint256 feePaid);

    // @notice Governance has updated the registrar parameters
    event ParamsUpdated(RegistrarParams oldParams, RegistrarParams newParams);

    // @dev restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (msg.sender != caller) revert UnauthorizedAction();
        _;
    }

    constructor(address admin) {
        _admin = admin;
    }

    /*
     * @dev receive function for UniswapV2Router02, collects all dust native tokens.
     */
    receive() external payable {}

    // @notice Initializes the registrar. Can only be called by the admin
    function init(RegistrarParams memory p) public restrict(_admin) {
        _params = p;
        _admin = address(0);
    }

    // @notice Returns the set params
    function params() public view returns (RegistrarParams memory) {
        return _params;
    }

    // @notice This serves as gas abstraction for registering assets on Hyperbridge
    // by collecting fees here. The asset metadata still needs to be provided
    // on Hyperbridge, but by paying here. It can be provided as an unsigned
    // extrinsic.
    //
    // @dev Collects the registration fees in either the native token or IIsmpHost.feeToken.
    // The resulting request must be relayed to Hyperbridge as this module provides no refunds.
    function registerAsset(bytes32 assetId) public payable {
        address feeToken = IIsmpHost(_params.host).feeToken();
        uint256 messagingFee = 64 * IIsmpHost(_params.host).perByteFee(bytes(""));
        uint256 baseFee = _params.baseFee;
        uint256 fee = baseFee + messagingFee;

        // user has provided the native token
        if (msg.value > 0) {
            address uniswapV2 = IIsmpHost(_params.host).uniswapV2Router();
            address[] memory path = new address[](2);
            path[0] = IUniswapV2Router02(uniswapV2).WETH();
            path[1] = feeToken;
            IUniswapV2Router02(uniswapV2).swapETHForExactTokens{value: msg.value}(
                fee,
                path,
                address(this),
                block.timestamp
            );
            SafeERC20.safeTransfer(IERC20(feeToken), _params.host, baseFee);
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken), msg.sender, _params.host, baseFee);
            SafeERC20.safeTransferFrom(IERC20(feeToken), msg.sender, address(this), messagingFee);
        }
        bytes memory data = abi.encode(RequestBody({owner: msg.sender, assetId: assetId}));

        // approve the host with the exact amount
        SafeERC20.safeIncreaseAllowance(IERC20(feeToken), _params.host, fee);
        DispatchPost memory request = DispatchPost({
            dest: IIsmpHost(_params.host).hyperbridge(),
            to: bytes("registry"),
            body: data,
            timeout: 0,
            fee: 0,
            payer: msg.sender
        });
        IDispatcher(_params.host).dispatch(request);

        emit RegistrationBegun({assetId: assetId, owner: msg.sender, feePaid: baseFee});
    }

    // @notice Governance parameter updates
    function onAccept(IncomingPostRequest calldata incoming) external override restrict(_params.host) {
        // only hyperbridge can do this
        if (!incoming.request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        RegistrarParams memory update = abi.decode(incoming.request.body, (RegistrarParams));

        emit ParamsUpdated({oldParams: _params, newParams: update});

        _params = update;
    }
}
