// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.17;

//  ==========  External imports    ==========
import {ERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "forge-std/console.sol";

contract MiniStaking is ERC20 {
    uint8 constant _decimals = 18;
    address stakingAddress;

    constructor(address stakingToken) ERC20("MiniStaking", "MINIs") {
        stakingAddress = stakingToken;
    }

    function decimals() public pure override returns (uint8) {
        return _decimals;
    }

    function recordStake(address beneficary) external {
        console.log("recordStake called");
        uint256 currentBalance = IERC20(stakingAddress).balanceOf(address(this));
        console.log("currentBalance: ", currentBalance);
        _mint(beneficary, currentBalance);
        console.log("minted: ", currentBalance);
    }
}
