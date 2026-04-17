// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2, Params} from "../src/apps/IntentGatewayV2.sol";
import {SolverAccount} from "../src/utils/SolverAccount.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {VWAPOracle} from "../src/utils/VWAPOracle.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        // Load optional configuration from environment
        bool solverSelection = vm.envOr("SOLVER_SELECTION", false);
        uint256 surplusShareBps = vm.envOr("SURPLUS_SHARE_BPS", uint256(5000)); // 50% default
        uint256 protocolFeeBps = vm.envOr("PROTOCOL_FEE_BPS", uint256(0)); // 0% default

        // Check if we should deploy CallDispatcher and VWAPOracle
        bool deployCallDispatcher = vm.envOr("DEPLOY_CALL_DISPATCHER", true);
        bool deployVWAPOracle = vm.envOr("DEPLOY_VWAP_ORACLE", true);

        // Deploy CallDispatcher if needed
        address callDispatcherAddr;
        if (deployCallDispatcher) {
            CallDispatcher callDispatcher = new CallDispatcher{salt: salt}();
            callDispatcherAddr = address(callDispatcher);
            console.log("CallDispatcher deployed at:", callDispatcherAddr);
        } else {
            callDispatcherAddr = config.get("CALL_DISPATCHER").toAddress();
            console.log("Using existing CallDispatcher at:", callDispatcherAddr);
        }

        // Deploy VWAPOracle (Price Oracle) if needed
        address vwapOracleAddr;
        if (deployVWAPOracle) {
            VWAPOracle vwapOracle = new VWAPOracle{salt: salt}(admin);
            vwapOracleAddr = address(vwapOracle);
            console.log("VWAPOracle deployed at:", vwapOracleAddr);
        } else {
            vwapOracleAddr = vm.envOr("PRICE_ORACLE", address(0));
            console.log("Using existing VWAPOracle at:", vwapOracleAddr);
        }

        // Deploy IntentGatewayV2
        IntentGatewayV2 intentGatewayV2 = new IntentGatewayV2{salt: salt}(admin);
        console.log("IntentGatewayV2 deployed at:", address(intentGatewayV2));

        // Set parameters on IntentGatewayV2
        intentGatewayV2.setParams(
            Params({
                host: HOST_ADDRESS,
                dispatcher: callDispatcherAddr,
                solverSelection: solverSelection,
                surplusShareBps: surplusShareBps,
                protocolFeeBps: protocolFeeBps,
                priceOracle: vwapOracleAddr
            })
        );

        // Deploy SolverAccount
        SolverAccount solverAccount = new SolverAccount{salt: salt}(address(intentGatewayV2));
        console.log("SolverAccount deployed at:", address(solverAccount));

        // Update config
        if (deployCallDispatcher) {
            config.set("CALL_DISPATCHER", callDispatcherAddr);
        }
        if (deployVWAPOracle) {
            config.set("VWAP_ORACLE", vwapOracleAddr);
        }
        config.set("INTENT_GATEWAY_V2", address(intentGatewayV2));
        config.set("SOLVER_ACCOUNT", address(solverAccount));

        console.log("");
        console.log("=== Deployment Summary ===");
        console.log("CallDispatcher:", callDispatcherAddr);
        console.log("VWAPOracle:", vwapOracleAddr);
        console.log("IntentGatewayV2:", address(intentGatewayV2));
        console.log("SolverAccount:", address(solverAccount));

        if (deployVWAPOracle) {
            console.log("");
            console.log("=== IMPORTANT: Post-deployment step ===");
            console.log("VWAPOracle needs initialization. Call VWAPOracle.init() with:");
            console.log("  - hostAddr:", HOST_ADDRESS);
            console.log("  - updates: TokenDecimalsUpdate[] for token decimals on remote chains");
        }
    }
}
