// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster} from "../src/utils/SimplexPaymaster.sol";
import {BaseScript} from "./BaseScript.sol";

/// @notice Deploys a new SimplexPaymaster implementation only. The live ERC-1967 proxy keeps its
/// address; Hyperbridge governance points it at this implementation through the
/// intents-coprocessor pallet's `upgrade_paymaster`, with `migrate()` as the init data so the
/// proxy's EntryPoint deposit moves to v0.9 and its v0.8 stake unlocks in the same transaction.
contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        SimplexPaymaster implementation = new SimplexPaymaster{salt: salt}();

        vm.stopBroadcast();

        console.log("SimplexPaymaster implementation deployed at:", address(implementation));
        console.log("upgrade_paymaster init_data:", vm.toString(abi.encodeCall(SimplexPaymaster.migrate, ())));
        console.log("After the old stake's unstake delay, call withdrawStakeV08() to move it into the v0.9 deposit.");

        config.set("SIMPLEX_PAYMASTER_IMPL", address(implementation));
    }
}
