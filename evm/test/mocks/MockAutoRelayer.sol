// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";
import "openzeppelin/utils/math/Math.sol";
import "ismp/StateMachine.sol";
import {MockHost} from "./MockHost.sol";
import {PostRequest} from "ismp/IIsmp.sol";

contract MockAutoRelayer is Test {
    address chain_a_host;
    address chain_b_host;

    function set(address a_host, address b_host) external {
        chain_a_host = a_host;
        chain_b_host = b_host;
    }
    /**
     * @dev Dispatch an incoming post request to destination module
     * @param request - post request
     */

    function autoRelay(PostRequest memory request) external {
        vm.startPrank(address(this), address(this));
        MockHost(keccak256(request.dest) == keccak256(StateMachine.ethereum()) ? chain_a_host : chain_b_host)
            .dispatchIncoming(request);
    }
}

// global function
function _bytesToAddress(bytes memory _bytes) pure returns (address addr) {
    require(_bytes.length >= 20, "Invalid address length");
    assembly {
        addr := mload(add(_bytes, 20))
    }
}
