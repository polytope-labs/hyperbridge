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

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/// @notice Stateless ERC-20: every state change returns `true` without doing
/// anything; every read returns max-uint256. Set as the EvmHost `feeToken`
/// during the bandwidth-mode cutover so the host's existing
/// `safeTransfer*(feeToken, …)` paths collapse to zero-cost no-ops without
/// touching `EvmHost.sol`.
contract NoOpERC20 is IERC20 {
    string public constant name = "Hyperbridge NoOp";
    string public constant symbol = "HBNOOP";
    uint8 public constant decimals = 18;

    function totalSupply() external pure returns (uint256) {
        return type(uint256).max;
    }

    function balanceOf(address) external pure returns (uint256) {
        return type(uint256).max;
    }

    function allowance(address, address) external pure returns (uint256) {
        return type(uint256).max;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        emit Transfer(msg.sender, to, amount);
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        emit Transfer(from, to, amount);
        return true;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        emit Approval(msg.sender, spender, amount);
        return true;
    }
}
