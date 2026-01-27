// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "@openzeppelin/contracts/utils/Strings.sol";
import "stringutils/strings.sol";

import {EvmHost, HostParams} from "../src/core/EvmHost.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {BaseScript} from "./BaseScript.sol";
import "../src/core/HandlerV1.sol";

import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";
import {SP1Verifier} from "@sp1-contracts/v5.0.0/SP1VerifierGroth16.sol";
import {MultiProofClient} from "../src/consensus/MultiProofClient.sol";
import {IConsensus} from "@hyperbridge/core/interfaces/IConsensus.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // Deploy consensus clients
        BeefyV1 beefyV1 = new BeefyV1{salt: salt}();
        console.log("BeefyV1 deployed at:", address(beefyV1));

        SP1Verifier verifier = new SP1Verifier{salt: salt}();
        console.log("SP1Verifier deployed at:", address(verifier));

        SP1Beefy sp1 = new SP1Beefy{salt: salt}(verifier, sp1VerificationKey);
        console.log("SP1Beefy deployed at:", address(sp1));

        MultiProofClient consensusClient = new MultiProofClient{salt: salt}(IConsensus(sp1), IConsensus(beefyV1));
        console.log("MultiProofClient deployed at:", address(consensusClient));

        // Update host params if not mainnet
        bool isMainnet = config.get("is_mainnet").toBool();
        console.log("Is mainnet:", isMainnet);

        if (!isMainnet) {
            HostParams memory params = EvmHost(HOST_ADDRESS).hostParams();
            params.consensusClient = address(consensusClient);
            EvmHost(HOST_ADDRESS).updateHostParams(params);
            console.log("Host params updated with new consensus client");
        }
    }
}
