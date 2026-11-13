// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";

import {HostManagerParams} from "../src/core/HostManager.sol";
import {OwnableHandler} from "../src/core/OwnableHandler.sol";
import {OwnableHostManager} from "../src/core/OwnableHostManager.sol";
import {BaseScript} from "./BaseScript.sol";

/**
 * @notice Deploys an OwnableHandler and an OwnableHostManager for a host that already exists, both
 * owned by `OWNER`. The manager is bound to the host at construction and keeps the governance
 * relayer as its admin, so governance from Hyperbridge still reaches it. The host is pointed at
 * them afterwards through a `SetHostParam` governance request, which the HostManager currently in
 * place delivers.
 */
contract DeployOwnableHandlerAndHostManager is BaseScript {
    function deploy() internal override {
        address owner = vm.envAddress("OWNER");
        address relayer = vm.envAddress("GOVERNANCE_RELAYER");

        OwnableHandler ownableHandler = new OwnableHandler{salt: salt}(owner);
        OwnableHostManager ownableHostManager = new OwnableHostManager{salt: salt}(
            owner, HostManagerParams({admin: relayer, host: HOST_ADDRESS})
        );
        vm.stopBroadcast();

        console.log("OwnableHandler deployed at:", address(ownableHandler));
        console.log("OwnableHostManager deployed at:", address(ownableHostManager));
        console.log("Owner:", owner);
        console.log("Governance relayer (admin):", relayer);
        config.set("OWNABLE_HANDLER", address(ownableHandler));
        config.set("OWNABLE_HOST_MANAGER", address(ownableHostManager));
    }
}
