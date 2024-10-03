// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "@openzeppelin/contracts/utils/Strings.sol";
import "stringutils/strings.sol";

import {EvmHost, HostParams} from "../src/hosts/EvmHost.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {BaseScript} from "./BaseScript.sol";
import "../src/modules/HandlerV1.sol";

import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";
import {SP1Verifier} from "@sp1-contracts/v2.0.0/SP1VerifierPlonk.sol";
import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";

contract DeployScript is BaseScript {
    using strings for *;


    function run() external {
        vm.startBroadcast(uint256(privateKey));

        // SP1Verifier verifier = new SP1Verifier();
        // SP1Beefy consensusClient = new SP1Beefy(verifier);
        HandlerV1 handler = new HandlerV1();

        if (equal(host, "sepolia") || host.toSlice().startsWith("eth".toSlice())) {
            HostParams memory params = EvmHost(SEPOLIA_HOST).hostParams();
            // params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(SEPOLIA_HOST).updateHostParams(params);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            HostParams memory params = EvmHost(ARB_SEPOLIA_HOST).hostParams();
            // params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(ARB_SEPOLIA_HOST).updateHostParams(params);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            HostParams memory params = EvmHost(OP_SEPOLIA_HOST).hostParams();
            // params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(OP_SEPOLIA_HOST).updateHostParams(params);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            HostParams memory params = EvmHost(BASE_SEPOLIA_HOST).hostParams();
            // params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(BASE_SEPOLIA_HOST).updateHostParams(params);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            HostParams memory params = EvmHost(BSC_TESTNET_HOST).hostParams();
            // params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(BSC_TESTNET_HOST).updateHostParams(params);
        } else if (host.toSlice().startsWith("chiado".toSlice())) {
            HostParams memory params = EvmHost(CHIADO_HOST).hostParams();
            // params.consensusClient = address(consensusClient);
            params.handler = address(handler);
            EvmHost(CHIADO_HOST).updateHostParams(params);
        } else {
            revert("Unknown Host");
        }
    }
}
