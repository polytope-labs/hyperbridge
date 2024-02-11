// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "./EvmHost.sol";
import "ismp/StateMachine.sol";

contract ArbitrumHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    function host() public pure override returns (bytes memory) {
        return StateMachine.arbitrum();
    }
}
