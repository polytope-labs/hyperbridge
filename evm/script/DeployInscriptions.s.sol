// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {CrossChainInscription} from "../src/modules/Inscriptions.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        vm.startBroadcast(uint256(privateKey));
        CrossChainInscription module = new CrossChainInscription{salt: salt}(admin);

        if (host.toSlice().startsWith("ethereum".toSlice())) {
            module.setHost(ETHEREUM_HOST);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            module.setHost(ARBITRUM_HOST);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            module.setHost(OPTIMISM_HOST);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            module.setHost(BASE_HOST);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            module.setHost(BNB_HOST);
        } else if (host.toSlice().startsWith("gnosis".toSlice())) {
            module.setHost(GNOSIS_HOST);
        }

        vm.stopBroadcast();
    }
}
