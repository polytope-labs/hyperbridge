// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;


struct CallDispatcherParams {
    address target;
    bytes data;
}


interface ICallDispatcher {
    function dispatch(CallDispatcherParams memory params) external returns (bytes memory result, bool success);
}
