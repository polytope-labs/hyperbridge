// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "./EvmHost.sol";
import "ismp/StateMachine.sol";

contract BaseHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    /// chainId for the base mainnet
    uint256 public constant CHAIN_ID = 8453;

    function chainId() public pure override returns (uint256) {
        return CHAIN_ID;
    }

    function host() public pure override returns (bytes memory) {
        return StateMachine.base();
    }
}
