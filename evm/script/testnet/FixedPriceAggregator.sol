// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

/// @notice Testnet only: a Chainlink AggregatorV3 stand-in that reports a fixed price which never goes stale.
contract FixedPriceAggregator {
    int256 public immutable answer;
    uint8 public immutable decimals;

    constructor(int256 answer_, uint8 decimals_) {
        answer = answer_;
        decimals = decimals_;
    }

    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 price, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        return (1, answer, block.timestamp, block.timestamp, 1);
    }
}
