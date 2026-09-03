// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {HostManager, HostManagerParams} from "../src/core/HostManager.sol";
import {BaseScript} from "./BaseScript.sol";

/// @notice Deploys a replacement HostManager for a host that already exists, bound to that host
/// and to the governance relayer. The host is pointed at it afterwards through a `SetHostParam`
/// governance request, which the HostManager currently in place delivers.
contract DeployHostManager is BaseScript {
    using strings for *;

    function deploy() internal override {
        address relayer = vm.envAddress("GOVERNANCE_RELAYER");

        HostManager manager = new HostManager{salt: salt}(HostManagerParams({admin: admin, host: address(0)}));
        manager.setIsmpHost(HOST_ADDRESS);
        // Requires the broadcaster to be the host admin.
        manager.setRelayer(relayer);

        vm.stopBroadcast();

        console.log("HostManager deployed at:", address(manager));
        console.log("Governance relayer:", relayer);

        config.set("HOST_MANAGER", address(manager));
    }
}
