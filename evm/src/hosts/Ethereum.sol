// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "../EvmHost.sol";
import "ismp/StateMachine.sol";

contract EthereumHost is EvmHost {
    constructor(HostParams memory params) EvmHost(params) {}

    function host() public override returns (bytes memory) {
        return StateMachine.ethereum();
    }
}
