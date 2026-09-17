// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2} from "../src/apps/IntentGatewayV2.sol";
import {ExtrinsicIntents} from "../src/apps/intentsv2/ExtrinsicIntents.sol";
import {IntrinsicModule} from "../src/apps/intentsv2/IntrinsicModule.sol";
import {ExtrinsicModule} from "../src/apps/intentsv2/ExtrinsicModule.sol";
import {BaseScript} from "./BaseScript.sol";

/// @dev Initialization payload for an upgrade of an existing proxy: `migrate(owner)` for a proxy
/// at 2 or owner-layout 3, nothing for one already at 4.
function intentGatewayUpgradeInitialization(IntentGatewayV2 gateway, address owner) view returns (bytes memory) {
    uint64 current = gateway.version();
    if (current == 4) return bytes("");
    require(current == 2 || current == 3, "Unsupported IntentGateway version");
    if (current == 2) require(owner != address(0), "GATEWAY_OWNER is unset");
    return abi.encodeCall(IntentGatewayV2.migrate, (owner));
}

/// @notice Shared by the IntentGatewayV2 deploy scripts: modules first, then the implementation.
abstract contract IntentGatewayScript is BaseScript {
    using strings for *;

    /**
     * @dev Deploys the modules and the implementation via CREATE2 with the script's salt. A module
     * already at its address is reused. Prints the `data` for `execute_on_gateway`.
     */
    function _deployImplementation() internal returns (IntentGatewayV2 implementation) {
        // Query the configured proxy before broadcasting any deployments. New chains initialize
        // their fresh proxy separately; existing proxies advance through version-aware migration.
        vm.stopBroadcast();
        bool hasProxy = config.exists("INTENT_GATEWAY_V2");
        bytes memory migration;
        if (hasProxy) {
            migration = intentGatewayUpgradeInitialization(
                IntentGatewayV2(payable(config.get("INTENT_GATEWAY_V2").toAddress())),
                vm.envOr("GATEWAY_OWNER", address(0))
            );
        }
        vm.startBroadcast(uint256(privateKey));
        address intrinsic = vm.computeCreate2Address(salt, keccak256(type(IntrinsicModule).creationCode));
        if (intrinsic.code.length == 0) intrinsic = address(new IntrinsicModule{salt: salt}());

        address extrinsic = vm.computeCreate2Address(salt, keccak256(type(ExtrinsicModule).creationCode));
        if (extrinsic.code.length == 0) extrinsic = address(new ExtrinsicModule{salt: salt}());

        address predicted = vm.computeCreate2Address(
            salt, keccak256(abi.encodePacked(type(IntentGatewayV2).creationCode, abi.encode(intrinsic, extrinsic)))
        );
        require(predicted.code.length == 0, "IntentGatewayV2 implementation already deployed for this VERSION");
        implementation = new IntentGatewayV2{salt: salt}(intrinsic, extrinsic);

        vm.stopBroadcast();

        console.log("IntrinsicModule at:", intrinsic);
        console.log("ExtrinsicModule at:", extrinsic);
        console.log("IntentGatewayV2 implementation deployed at:", address(implementation));
        if (hasProxy) {
            // `execute_on_gateway(data)` prepends the `Execute` discriminator itself.
            console.log("execute_on_gateway data (version-aware upgradeToAndCall):");
            console.logBytes(abi.encodeCall(ExtrinsicIntents.upgradeToAndCall, (address(implementation), migration)));
        }
        vm.startBroadcast(uint256(privateKey));
    }

    /**
     * @dev Records the module and implementation addresses in the config. Call after the broadcast.
     */
    function _recordImplementation(IntentGatewayV2 implementation) internal {
        config.set("INTENT_GATEWAY_V2_INTRINSIC_MODULE", implementation.intrinsicModule());
        config.set("INTENT_GATEWAY_V2_EXTRINSIC_MODULE", implementation.extrinsicModule());
        config.set("INTENT_GATEWAY_V2_IMPL", address(implementation));
    }
}
