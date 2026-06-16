// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "@openzeppelin/contracts/utils/Strings.sol";
import "stringutils/strings.sol";

import {EvmHost, HostParams} from "../src/core/EvmHost.sol";
import {EcdsaBeefy} from "../src/consensus/EcdsaBeefy.sol";
import {BaseScript} from "./BaseScript.sol";
import {HandlerV2} from "../src/core/HandlerV2.sol";
import {BandwidthManager} from "../src/apps/BandwidthManager.sol";

import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";
import {SP1Verifier} from "@sp1-contracts/v6.1.0/SP1VerifierGroth16.sol";
import {ConsensusRouter} from "../src/consensus/ConsensusRouter.sol";
import {IConsensusV2} from "@hyperbridge/core/interfaces/IConsensusV2.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // Deploy consensus clients
        EcdsaBeefy ecdsaBeefy = new EcdsaBeefy{salt: salt}();
        SP1Verifier verifier = new SP1Verifier{salt: salt}();
        SP1Beefy sp1 = new SP1Beefy{salt: salt}(verifier, sp1VerificationKey);
        ConsensusRouter consensusClient = new ConsensusRouter{salt: salt}(
            IConsensusV2(sp1),
            IConsensusV2(ecdsaBeefy)
        );

        // HandlerV2 handler = new HandlerV2{salt: salt}();

        // BandwidthManager bandwidthManager = new BandwidthManager{salt: salt}(admin);
        // bandwidthManager.setHost(HOST_ADDRESS);
        // bandwidthManager.renounceOwnership();

        // Update host params if not mainnet
        bool isMainnet = config.get("is_mainnet").toBool();
        if (!isMainnet) {
            HostParams memory params = EvmHost(HOST_ADDRESS).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(HOST_ADDRESS).updateHostParams(params);
            console.log("Host params updated with new consensus client and handler");
        }

        vm.stopBroadcast();
        console.log("Is mainnet:", isMainnet);
        console.log("EcdsaBeefy deployed at:", address(ecdsaBeefy));
        console.log("SP1Verifier deployed at:", address(verifier));
        console.log("SP1Beefy deployed at:", address(sp1));
        console.log("ConsensusRouter deployed at:", address(consensusClient));
        // console.log("HandlerV2 deployed at:", address(handler));
        // console.log("BandwidthManager deployed at:", address(bandwidthManager));
        config.set("ECDSA_BEEFY", address(ecdsaBeefy));
        config.set("SP1_VERIFIER", address(verifier));
        config.set("SP1_BEEFY", address(sp1));
        config.set("CONSENSUS_ROUTER", address(consensusClient));
        // config.set("BANDWIDTH_MANAGER", address(bandwidthManager));
        // config.set("HANDLER_V2", address(handler));
    }
}
