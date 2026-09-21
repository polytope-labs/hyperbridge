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

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title HyperCNGN
 * @author Polytope Labs (hello@polytope.technology)
 * @notice A testnet stand-in for cNGN, used to exercise the FX filler. Six decimals, like cNGN.
 */
contract HyperCNGN is ERC20, Ownable {
    /**
     * @param owner Receives the initial supply and may mint more.
     * @param initialSupply Amount minted to `owner`, in base units.
     */
    constructor(address owner, uint256 initialSupply) ERC20("hyperCNGN", "hCNGN") Ownable(owner) {
        _mint(owner, initialSupply);
    }

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    /**
     * @notice Mints `amount` to `to`. Owner only.
     */
    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }
}
