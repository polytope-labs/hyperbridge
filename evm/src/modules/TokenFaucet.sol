// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {IERC6160Ext20} from "ERC6160/interfaces/IERC6160Ext20.sol";

/// Allows access to a fixed amount of tokens to users on a daily basis
contract TokenFaucet {
    mapping(address => uint256) private consumers;
    address private token;

    constructor(address _token) {
        token = _token;
    }

    /// Will only drip tokens, once per day
    function drip() public {
        uint256 lastDrip = consumers[msg.sender];
        uint256 delay = block.timestamp - lastDrip;

        if (delay < 86400) {
            revert("Can only request tokens once daily");
        }

        consumers[msg.sender] = block.timestamp;
        IERC6160Ext20(token).mint(msg.sender, 1000 * 1e18, "");
    }
}
