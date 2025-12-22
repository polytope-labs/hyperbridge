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

import {HyperFungibleTokenImpl} from "../src/apps/HyperFungibleTokenImpl.sol";

/**
 * @title FeeToken
 * @notice Test token that extends HyperFungibleTokenImpl with initial supply minting
 */
contract FeeToken is HyperFungibleTokenImpl {
    constructor(address admin, string memory name, string memory symbol) HyperFungibleTokenImpl(admin, name, symbol) {
        // Mint initial supply to tx.origin for testing purposes
        _mint(tx.origin, 1_000_000_000_000000000000000000);
    }
}
