// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster, IStakeManager} from "../../src/utils/SimplexPaymaster.sol";
import {BaseScript} from "../BaseScript.sol";

/// @notice Tops the SIMPLEX_PAYMASTER EntryPoint deposit and stake up to PAYMASTER_DEPOSIT and
///         PAYMASTER_STAKE (wei). Sends nothing for a balance already at its target, so a re-run
///         is a no-op. The stake goes through `addStake`, which only the treasury may call.
contract DeployScript is BaseScript {
    using strings for *;

    uint32 internal constant UNSTAKE_DELAY = 1 days;

    function deploy() internal override {
        SimplexPaymaster paymaster = SimplexPaymaster(payable(config.get("SIMPLEX_PAYMASTER").toAddress()));
        uint256 depositTarget = vm.envUint("PAYMASTER_DEPOSIT");
        uint256 stakeTarget = vm.envUint("PAYMASTER_STAKE");
        IStakeManager entryPoint = IStakeManager(address(paymaster.entryPoint()));

        IStakeManager.DepositInfo memory info = entryPoint.getDepositInfo(address(paymaster));
        if (info.deposit < depositTarget) {
            paymaster.deposit{value: depositTarget - info.deposit}();
        }
        if (info.stake < stakeTarget || info.unstakeDelaySec < UNSTAKE_DELAY) {
            require(paymaster.treasury() == vm.addr(uint256(privateKey)), "PRIVATE_KEY is not the paymaster treasury");
            uint256 topUp = info.stake < stakeTarget ? stakeTarget - info.stake : 0;
            paymaster.addStake{value: topUp}(UNSTAKE_DELAY);
        }

        vm.stopBroadcast();

        info = entryPoint.getDepositInfo(address(paymaster));
        console.log("SimplexPaymaster:", address(paymaster));
        console.log("  EntryPoint:", address(entryPoint));
        console.log("  deposit:", info.deposit);
        console.log("  stake:", info.stake);
        console.log("  unstakeDelaySec:", info.unstakeDelaySec);
    }
}
