// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";

contract FeeToken is ERC6160Ext20 {
    constructor(address _defaultOwner, string memory _name, string memory _symbol)
        ERC6160Ext20(_defaultOwner, _name, _symbol)
    {
        _mint(tx.origin, 1_000_000_000_000000000000000000);
    }

    function superApprove(address owner, address spender) public {
        _approve(owner, spender, type(uint256).max);
    }
}
