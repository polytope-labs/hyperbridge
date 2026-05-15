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
import {IConsensus} from "@hyperbridge/core/interfaces/IConsensus.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // Deploy consensus clients
        EcdsaBeefy ecdsaBeefy = new EcdsaBeefy{salt: salt}();
        console.log("EcdsaBeefy deployed at:", address(ecdsaBeefy));

        SP1Verifier verifier = new SP1Verifier{salt: salt}();
        console.log("SP1Verifier deployed at:", address(verifier));

        SP1Beefy sp1 = new SP1Beefy{salt: salt}(verifier, sp1VerificationKey);
        console.log("SP1Beefy deployed at:", address(sp1));

        ConsensusRouter consensusClient = new ConsensusRouter{salt: salt}(
            IConsensus(sp1),
            IConsensus(ecdsaBeefy)
        );
        console.log("ConsensusRouter deployed at:", address(consensusClient));

        HandlerV2 handler = new HandlerV2{salt: salt}();
        console.log("HandlerV2 deployed at:", address(handler));

        BandwidthManager bandwidthManager = new BandwidthManager{salt: salt}(admin);
        bandwidthManager.setHost(HOST_ADDRESS);
        bandwidthManager.renounceOwnership();
        console.log("BandwidthManager deployed at:", address(bandwidthManager));

        // Update host params if not mainnet
        bool isMainnet = config.get("is_mainnet").toBool();
        console.log("Is mainnet:", isMainnet);

        if (!isMainnet) {
            HostParams memory params = EvmHost(HOST_ADDRESS).hostParams();
            params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(HOST_ADDRESS).updateHostParams(params);
            console.log("Host params updated with new consensus client and handler");
        }

        vm.stopBroadcast();
        config.set("ECDSA_BEEFY", address(ecdsaBeefy));
        config.set("SP1_BEEFY", address(sp1));
        config.set("CONSENSUS_ROUTER", address(consensusClient));
        config.set("BANDWIDTH_MANAGER", address(bandwidthManager));
        config.set("HANDLER_V2", address(handler));
    }
}
