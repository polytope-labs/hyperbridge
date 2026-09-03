// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {BaseScript} from "./BaseScript.sol";
import {EvmHost} from "../src/core/EvmHost.sol";
import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";
import {ConsensusRouter} from "../src/consensus/ConsensusRouter.sol";
import {IConsensusV2} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

/**
 * @notice Deploys a new SP1Beefy and a ConsensusRouter fronting it, reusing the EcdsaBeefy
 * client and the SP1 Groth16 verifier that are already deployed.
 *
 * @dev Only SP1Beefy's bytecode changed, so redeploying EcdsaBeefy and SP1Verifier is not just
 * unnecessary, it is impossible: with unchanged bytecode and the same VERSION salt, CREATE2
 * resolves to the addresses they already occupy and the deployment reverts. That is why this
 * script reads them from config rather than constructing them (`DeployHostUpdates` constructs
 * both and so cannot be re-run against an existing deployment).
 *
 * This script only publishes contracts — it does not activate them. `EvmHost.updateHostParams`
 * is restricted to the HostManager, which acts only on a governance request from the Hyperbridge
 * chain, so the host keeps using the old router until that request is executed.
 */
contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        address ecdsaBeefy = config.get("ECDSA_BEEFY").toAddress();
        address sp1Verifier = config.get("SP1_VERIFIER").toAddress();

        // Guard against wiring the router to an address that holds no code on this chain.
        require(ecdsaBeefy.code.length != 0, "ECDSA_BEEFY has no code on this chain");
        require(sp1Verifier.code.length != 0, "SP1_VERIFIER has no code on this chain");

        SP1Beefy sp1Beefy = new SP1Beefy{salt: salt}(ISP1Verifier(sp1Verifier), sp1VerificationKey);
        ConsensusRouter consensusRouter =
            new ConsensusRouter{salt: salt}(IConsensusV2(address(sp1Beefy)), IConsensusV2(ecdsaBeefy));

        vm.stopBroadcast();

        console.log("Reused EcdsaBeefy:       ", ecdsaBeefy);
        console.log("Reused SP1Verifier:      ", sp1Verifier);
        console.log("New SP1Beefy:            ", address(sp1Beefy));
        console.log("New ConsensusRouter:     ", address(consensusRouter));
        console.log("Host consensusClient is: ", EvmHost(HOST_ADDRESS).hostParams().consensusClient);
        console.log("^ unchanged until governance points the host at the new router");

        config.set("SP1_BEEFY", address(sp1Beefy));
        config.set("CONSENSUS_ROUTER", address(consensusRouter));
    }
}
