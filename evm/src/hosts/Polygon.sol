// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "./EvmHost.sol";
import "ismp/StateMachine.sol";

contract PolygonHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    /// chainId for the polygon mainnet
    uint256 public constant CHAIN_ID = 137;

    function chainId() public pure override returns (uint256) {
        return CHAIN_ID;
    }

    function host() public pure override returns (bytes memory) {
        return StateMachine.polygon();
    }
}
