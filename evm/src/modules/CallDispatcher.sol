// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import {ICallDispatcher} from "../interfaces/ICallDispatcher.sol";

/// @notice This contract is used to dispatch calls to other contracts.
contract CallDispatcher is ICallDispatcher {
    /// @dev funtion returns `success = false` if the target is not a contract and reverts if the call to the target contract fails.
    function dispatch(address target, bytes calldata data) external returns (bytes memory result, bool success) {
        uint32 size;
        assembly {
            size := extcodesize(target)
        }

        if (size > 0) {
            (success, result) = target.call(data);
            require(success, string(result));
            return (result, success);
        }
    }
}
