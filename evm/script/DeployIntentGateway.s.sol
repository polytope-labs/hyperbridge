// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2, Params, InitParams} from "../src/apps/IntentGatewayV2.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {IntentGatewayScript} from "./IntentGatewayScript.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {SolverAccount} from "../src/apps/intentsv2/SolverAccount.sol";
import {VWAPOracle} from "../src/utils/VWAPOracle.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

contract DeployScript is IntentGatewayScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // The implementation is always new; a module already at its CREATE2 address is reused.
        // The proxy is deployed only on a chain that
        // has none yet: where `INTENT_GATEWAY_V2` is already in the config, governance moves that
        // proxy to this implementation (`upgrade_gateway` for a proxy still on the pre-`Execute`
        // code, `execute_on_gateway` with `upgradeToAndCall` calldata afterwards), and only the
        // solver account is redeployed alongside.
        IntentGatewayV2 implementation = _deployImplementation();
        IntentGatewayV2 intentGateway;
        if (config.exists("INTENT_GATEWAY_V2")) {
            intentGateway = IntentGatewayV2(payable(config.get("INTENT_GATEWAY_V2").toAddress()));
        } else {
            intentGateway = _deployProxy(implementation);
        }
        SolverAccount solverAccount = new SolverAccount{salt: salt}(address(intentGateway));

        vm.stopBroadcast();

        console.log("IntentGateway proxy at:", address(intentGateway));
        console.log("SolverAccount deployed at:", address(solverAccount));

        _recordImplementation(implementation);
        config.set("INTENT_GATEWAY_V2", address(intentGateway));
        config.set("SOLVER_ACCOUNT", address(solverAccount));
    }

    /// @dev Proxy via CREATE2 with the same salt, initialized atomically through its init data,
    /// which arms the relayer gate from `GATEWAY_RELAYER` and sets the owner from `GATEWAY_OWNER`.
    /// The peer registry is passed by chain id only, since `initialize` binds each to
    /// `address(this)`, so the address depends on (impl address, salt, params, peer chain ids,
    /// relayer, owner), all identical across chains.
    function _deployProxy(IntentGatewayV2 implementation) internal returns (IntentGatewayV2) {
        address relayer = vm.envAddress("GATEWAY_RELAYER");
        require(relayer != address(0), "GATEWAY_RELAYER is unset");
        address owner = vm.envAddress("GATEWAY_OWNER");
        require(owner != address(0), "GATEWAY_OWNER is unset");
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

        InitParams memory init = InitParams({
            params: Params({
                host: HOST_ADDRESS,
                dispatcher: config.get("CALL_DISPATCHER").toAddress(),
                solverSelection: config.get("7702").toBool(),
                surplusShareBps: 6_000, // 60%
                protocolFeeBps: 5, // 0.05%
                priceOracle: address(0)
            }),
            peerChains: peerChains,
            relayer: relayer,
            owner: owner
        });
        bytes memory initData = abi.encodeCall(IntentGatewayV2.initialize, (init));
        ERC1967Proxy proxy = new ERC1967Proxy{salt: salt}(address(implementation), initData);
        console.log("IntentGateway relayer:", relayer);
        console.log("IntentGateway owner:", owner);
        return IntentGatewayV2(payable(address(proxy)));
    }
}
