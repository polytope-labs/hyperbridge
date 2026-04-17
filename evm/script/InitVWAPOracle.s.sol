// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {VWAPOracle} from "../src/utils/VWAPOracle.sol";
import {BaseScript} from "./BaseScript.sol";

contract InitVWAPOracleScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        address vwapOracleAddr = config.get("VWAP_ORACLE").toAddress();
        address intentGatewayAddr = config.get("INTENT_GATEWAY_V2").toAddress();

        VWAPOracle vwapOracle = VWAPOracle(vwapOracleAddr);

        // Initialize with empty token decimals for now
        // Token decimals can be updated later via Hyperbridge governance
        VWAPOracle.TokenDecimalsUpdate[] memory updates = new VWAPOracle.TokenDecimalsUpdate[](0);

        vwapOracle.init(HOST_ADDRESS, intentGatewayAddr, updates);

        console.log("VWAPOracle initialized at:", vwapOracleAddr);
        console.log("  host:", HOST_ADDRESS);
    }
}
