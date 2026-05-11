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

/**
 * @title IWETH
 * @notice Minimal interface for Wrapped Ether (WETH) contracts
 * @dev Used by WrappedHyperFungibleToken to support native token wrapping and unwrapping
 */
interface IWETH {
    /**
     * @notice Wraps native tokens into WETH
     * @dev Caller sends native tokens via msg.value, receives equivalent WETH balance
     */
    function deposit() external payable;

    /**
     * @notice Unwraps WETH back into native tokens
     * @dev Burns the specified amount of WETH and sends native tokens to the caller
     * @param amount The amount of WETH to unwrap
     */
    function withdraw(uint256 amount) external;
}
