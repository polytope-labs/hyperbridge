// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "multi-chain-tokens/interfaces/IERC6160Ext20.sol";

contract TokenFaucet {
    mapping(address => uint256) private consumers;
    address private token;

    constructor(address _token) {
        token = _token;
    }

    function drip() public {
        uint256 lastDrip = consumers[msg.sender];
        uint256 delay = block.timestamp - lastDrip;

        if (delay < 86400) {
            revert("Can only request tokens once daily");
        }

        consumers[msg.sender] = block.timestamp;
        IERC5679Ext20(token).mint(msg.sender, 100 * 10e18, "");
    }
}
