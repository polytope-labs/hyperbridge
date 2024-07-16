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
pragma solidity 0.8.17;

import {BaseIsmpModule, PostRequest, IncomingPostRequest} from "ismp/IIsmpModule.sol";
import {IDispatcher, DispatchPost} from "ismp/IDispatcher.sol";
import {IIsmpHost} from "ismp/IIsmpHost.sol";
import {IERC20} from "openzeppelin/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin/token/ERC20/utils/SafeERC20.sol";
import {IUniswapV2Router} from "../interfaces/IUniswapV2Router.sol";
import {Bytes} from "solidity-merkle-trees/trie/Bytes.sol";

struct AssetRegistration {
    // The asset identifier intended for registration
    bytes32 assetId;
    // The feetoken to use for fees
    address feeToken;
    // How much of the feeToken to swap for the hyperbridge feeToken
    uint256 amountToSwap;
}

struct RequestBody {
    // The asset owner
    address owner;
    // The assetId to create
    bytes32 assetId;
    // The base fee paid for registration, used in timeouts
    uint256 baseFee;
}

struct RegistrarParams {
    // The ERC20 contract address for the wrapped version of the local native token
    address erc20NativeToken;
    // Ismp host
    address host;
    // Local UniswapV2 contract address
    address uniswapV2;
    // registration base fee
    uint256 baseFee;
}

/**
 * @title The Token Registrar.
 * @author Polytope Labs
 *
 * @notice Serves as a form of gas abstraction for token
 * registration. By collecting fees on any chain and permitting token creation on the
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

    // A user has initiated the asset registration process
    event RegistrationBegun(bytes32 indexed assetId, address indexed owner);

    // Governance has updated the registrar parameters
    event ParamsUpdated(RegistrarParams oldParams, RegistrarParams newParams);

    // restricts call to the provided `caller`
    modifier restrict(address caller) {
        if (msg.sender != caller) revert UnauthorizedAction();
        _;
    }

    constructor(address admin) {
        _admin = admin;
    }

    function init(RegistrarParams memory p) public restrict(_admin) {
        _params = p;
        _admin = address(0);
    }

    // Returns the set params
    function params() public view returns (RegistrarParams memory) {
        return _params;
    }

    // This serves as gas abstraction for registering assets on Hyperbridge
    // by collecting fees here. The asset metadata still needs to be provided
    // on Hyperbridge, but by paying here. It can be provided as an unsigned
    // extrinsic.
    //
    // Collects the registration fees in any token that can be swapped for the
    // IIsmpHost.feeToken using the local UniswapV2 router. Any request must be
    // relayed to Hyperbridge as this module provides no refunds.
    function registerAsset(AssetRegistration memory registration) public payable {
        address feeToken = IIsmpHost(_params.host).feeToken();
        uint256 messagingFee = 96 * IIsmpHost(_params.host).perByteFee();
        uint256 fee = _params.baseFee + messagingFee;

        if (feeToken != registration.feeToken) {
            if (msg.value != 0) {
                (bool sent,) = _params.erc20NativeToken.call{value: msg.value}("");
                if (!sent) revert InconsistentState();
                registration.feeToken = _params.erc20NativeToken;
                registration.amountToSwap = msg.value;
            } else {
                SafeERC20.safeTransferFrom(
                    IERC20(registration.feeToken), msg.sender, address(this), registration.amountToSwap
                );
            }
            SafeERC20.safeIncreaseAllowance(IERC20(registration.feeToken), _params.uniswapV2, registration.amountToSwap);

            address[] memory path = new address[](2);
            path[0] = registration.feeToken;
            path[1] = feeToken;

            IUniswapV2Router(_params.uniswapV2).swapTokensForExactTokens(
                fee, registration.amountToSwap, path, address(this), block.timestamp
            );
            SafeERC20.safeTransfer(IERC20(feeToken), _params.host, _params.baseFee);
        } else {
            SafeERC20.safeTransferFrom(IERC20(feeToken), msg.sender, _params.host, _params.baseFee);
            SafeERC20.safeTransferFrom(IERC20(feeToken), msg.sender, address(this), messagingFee);
        }
        bytes memory data =
            abi.encode(RequestBody({owner: msg.sender, assetId: registration.assetId, baseFee: _params.baseFee}));

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

        emit RegistrationBegun({assetId: registration.assetId, owner: msg.sender});
    }

    // Governance parameter updates
    function onAccept(IncomingPostRequest calldata incoming) external override restrict(_params.host) {
        // only hyperbridge can do this
        if (!incoming.request.source.equals(IIsmpHost(_params.host).hyperbridge())) revert UnauthorizedAction();

        RegistrarParams memory update = abi.decode(incoming.request.body, (RegistrarParams));

        emit ParamsUpdated({oldParams: _params, newParams: update});

        _params = update;
    }
}
