// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster} from "../src/utils/SimplexPaymaster.sol";
import {BaseScript} from "./BaseScript.sol";

/// @notice Deploys a new SimplexPaymaster implementation only. The live ERC-1967 proxy keeps its
/// address; Hyperbridge governance points it at this implementation through the
/// intents-coprocessor pallet's `upgrade_paymaster`, with `migrate(relayer)` as the init data so
/// the relayer gate is armed in the same transaction.
contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        SimplexPaymaster implementation = new SimplexPaymaster{salt: salt}();

        vm.stopBroadcast();

        console.log("SimplexPaymaster implementation deployed at:", address(implementation));

        config.set("SIMPLEX_PAYMASTER_IMPL", address(implementation));
    }
}
