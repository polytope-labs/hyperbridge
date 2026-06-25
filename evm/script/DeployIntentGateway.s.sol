// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2, Params} from "../src/apps/IntentGatewayV2.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {BaseScript} from "./BaseScript.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {SolverAccount} from "../src/apps/intentsv2/SolverAccount.sol";
import {VWAPOracle} from "../src/utils/VWAPOracle.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // Deploy implementation and proxy via CREATE2 with the same salt. The proxy is initialized
        // atomically through its init data. The cross-chain peer registry is passed in by chain id
        // only — `initialize` binds each to `address(this)` — so no peer address is embedded in the
        // init data. The address depends on (impl address, salt, params, peer chain ids), all of
        // which are identical across chains, keeping the proxy address identical everywhere.
        address priceOracle = address(0);
        IntentGatewayV2 implementation = new IntentGatewayV2{salt: salt}(admin);
        bytes[] memory peerChains;
        if (config.get("is_mainnet").toBool()) {
            peerChains = new bytes[](9);
            peerChains[0] = StateMachine.evm(1); // ethereum
            peerChains[1] = StateMachine.evm(10); // optimism
            peerChains[2] = StateMachine.evm(42161); // arbitrum
            peerChains[3] = StateMachine.evm(8453); // base
            peerChains[4] = StateMachine.evm(56); // bsc
            peerChains[5] = StateMachine.evm(100); // gnosis
            peerChains[6] = StateMachine.evm(137); // polygon
            peerChains[7] = StateMachine.evm(420420419); // polkadot
            peerChains[8] = StateMachine.evm(1868); // soneium
        } else {
            peerChains = new bytes[](2);
            peerChains[0] = StateMachine.evm(97); // bsc testnet (chapel)
            peerChains[1] = StateMachine.evm(80002); // polygon amoy
        }

        bytes memory initData = abi.encodeCall(
            IntentGatewayV2.initialize,
            (
                Params({
                    host: HOST_ADDRESS,
                    dispatcher: config.get("CALL_DISPATCHER").toAddress(),
                    solverSelection: config.get("7702").toBool(),
                    surplusShareBps: 5_000, // 50%
                    protocolFeeBps: 30, // 0.3%
                    priceOracle: priceOracle
                }),
                peerChains
            )
        );
        ERC1967Proxy proxy = new ERC1967Proxy{salt: salt}(address(implementation), initData);
        IntentGatewayV2 intentGateway = IntentGatewayV2(payable(address(proxy)));
        SolverAccount solverAccount = new SolverAccount{salt: salt}(address(intentGateway));

        vm.stopBroadcast();

        console.log("IntentGateway implementation deployed at:", address(implementation));
        console.log("IntentGateway proxy deployed at:", address(intentGateway));
        console.log("SolverAccount deployed at:", address(solverAccount));

        config.set("INTENT_GATEWAY_V2", address(intentGateway));
        config.set("SOLVER_ACCOUNT", address(solverAccount));
        config.set("PRICE_ORACLE", address(priceOracle));
    }
}
