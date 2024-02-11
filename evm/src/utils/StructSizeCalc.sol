// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

struct Body {
    // amount to be sent
    uint256 amount;
    // The token identifier
    bytes32 tokenId;
    // sender address
    address from;
    // recipient address
    address to;
    // flag to redeem the erc20 asset on the destination
    bool redeem;
}

contract StructSizeCalc {
    function calculateStruct(Body memory body) external pure returns (uint256) {
        bytes memory data = abi.encode(body);

        return data.length;
    }
}