// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "ismp/EvmHost.sol";
import "ismp/interfaces/StateMachine.sol";

contract OptimismHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    function host() public override returns (bytes memory) {
        return StateMachine.optimism();
    }
}
