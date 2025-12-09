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
import {SP1Verifier} from "@sp1-contracts/v4.0.0-rc.3/SP1VerifierGroth16.sol";
import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        SP1Verifier verifier = new SP1Verifier();
        SP1Beefy consensusClient = new SP1Beefy(verifier);

        // HandlerV1 handler = new HandlerV1();
        // BeefyV1 consensusClient = new BeefyV1{salt: salt}();

        uint256 chainId = block.chainid;

        HostParams memory params = EvmHost(HOST_ADDRESS).hostParams();
        params.consensusClient = address(consensusClient);
        // params.handler = address(handler);
        EvmHost(HOST_ADDRESS).updateHostParams(params);
       
    }
}
