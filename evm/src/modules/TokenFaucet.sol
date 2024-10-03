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

import {IERC6160Ext20} from "@polytope-labs/erc6160/interfaces/IERC6160Ext20.sol";

/**
 * @title The TokenFaucet.
 * @author Polytope Labs (hello@polytope.technology)
 *
 * @notice Allows access to a fixed amount of tokens to users on a daily basis
 */
contract TokenFaucet {
    mapping(address => uint256) private consumers;

    // @dev Will only drip tokens, once per day
    function drip(address token) public {
        uint256 lastDrip = consumers[msg.sender];
        uint256 delay = block.timestamp - lastDrip;

        if (delay < 1 days) {
            revert("Can only request tokens once daily");
        }

        consumers[msg.sender] = block.timestamp;
        IERC6160Ext20(token).mint(msg.sender, 1000 * 1e18);
    }
}
