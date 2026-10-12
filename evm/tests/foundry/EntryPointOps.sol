// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {PackedUserOperation as EntryPointUserOp} from "@account-abstraction/contracts/interfaces/PackedUserOperation.sol";

/**
 * @dev `op` as the EntryPoint interface's own struct. OpenZeppelin's account and paymaster bases
 * declare an identical one, and Solidity never converts between two struct types.
 */
function toEntryPointOp(PackedUserOperation memory op) pure returns (EntryPointUserOp memory) {
    return abi.decode(abi.encode(op), (EntryPointUserOp));
}

/**
 * @dev `ops` as the EntryPoint interface's own struct, for `handleOps`.
 */
function toEntryPointOps(PackedUserOperation[] memory ops) pure returns (EntryPointUserOp[] memory) {
    return abi.decode(abi.encode(ops), (EntryPointUserOp[]));
}
