// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;



interface ICallDispatcher {
    function dispatch(address target, bytes calldata data) external returns (bytes memory result, bool success);
}