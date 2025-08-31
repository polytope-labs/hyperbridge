// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGateway, Params} from "../src/modules/IntentGateway.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        address admin = vm.envAddress("ADMIN");
        address dispatcher = vm.envAddress("DISPATCHER");

        vm.startBroadcast(uint256(privateKey));

        IntentGateway gateway = new IntentGateway{salt: salt}(admin);

        Params memory params = Params({host: address(0), dispatcher: dispatcher});

        // Set the host based on the current chain
        if (host.toSlice().startsWith("ethereum".toSlice())) {
            params.host = ETHEREUM_HOST;
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            params.host = ARBITRUM_HOST;
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            params.host = OPTIMISM_HOST;
        } else if (host.toSlice().startsWith("base".toSlice())) {
            params.host = BASE_HOST;
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            params.host = BNB_HOST;
        } else if (host.toSlice().startsWith("gnosis".toSlice())) {
            params.host = GNOSIS_HOST;
        } else if (host.toSlice().startsWith("soneium".toSlice())) {
            params.host = SONEIUM_HOST;
        } else if (host.toSlice().startsWith("polygon".toSlice())) {
            params.host = POLYGON_HOST;
        }

        gateway.setParams(params);

        vm.stopBroadcast();

        console.log("IntentGateway deployed at:", address(gateway));
    }
}
