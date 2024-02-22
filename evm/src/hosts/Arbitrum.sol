// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "./EvmHost.sol";
import "ismp/StateMachine.sol";

contract ArbitrumHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    /// chainId for the arbitrum mainnet
    uint256 public constant CHAIN_ID = 42161;

    function chainId() public pure override returns (uint256) {
        return CHAIN_ID;
    }

    function host() public pure override returns (bytes memory) {
        return StateMachine.arbitrum();
    }
}
