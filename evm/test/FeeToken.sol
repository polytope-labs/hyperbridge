// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {ERC20} from "openzeppelin/token/ERC20/ERC20.sol";

contract FeeToken is ERC20 {
    constructor(uint256 initialSupply) ERC20("Fee Token", "FTK") {
        _mint(tx.origin, initialSupply);
    }

    function superApprove(address owner, address spender) public {
        _approve(owner, spender, type(uint256).max);
    }
}
