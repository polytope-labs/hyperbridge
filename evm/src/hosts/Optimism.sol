// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "./EvmHost.sol";
import "ismp/StateMachine.sol";

contract OptimismHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    /// chainId for the optimism mainnet
    uint256 public constant CHAIN_ID = 10;

    function chainId() public pure override returns (uint256) {
        return CHAIN_ID;
    }

    function host() public pure override returns (bytes memory) {
        return StateMachine.optimism();
    }
}
