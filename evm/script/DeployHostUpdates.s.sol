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

        // Ethereum (mainnet: 1, sepolia: 11155111)
        if (chainId == 1 || chainId == 11155111) {
            HostParams memory params = EvmHost(ETHEREUM_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(ETHEREUM_HOST).updateHostParams(params);
        }
        // Arbitrum (mainnet: 42161, sepolia: 421614)
        else if (chainId == 42161 || chainId == 421614) {
            HostParams memory params = EvmHost(ARBITRUM_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(ARBITRUM_HOST).updateHostParams(params);
        }
        // Optimism (mainnet: 10, sepolia: 11155420)
        else if (chainId == 10 || chainId == 11155420) {
            HostParams memory params = EvmHost(OPTIMISM_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(OPTIMISM_HOST).updateHostParams(params);
        }
        // Base (mainnet: 8453, sepolia: 84532)
        else if (chainId == 8453 || chainId == 84532) {
            HostParams memory params = EvmHost(BASE_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(BASE_HOST).updateHostParams(params);
        }
        // BSC (mainnet: 56, testnet: 97)
        else if (chainId == 56 || chainId == 97) {
            HostParams memory params = EvmHost(BNB_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(BNB_HOST).updateHostParams(params);
        }
        // Gnosis (mainnet: 100, chiado: 10200)
        else if (chainId == 100 || chainId == 10200) {
            HostParams memory params = EvmHost(GNOSIS_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(GNOSIS_HOST).updateHostParams(params);
        }
        // Polygon (mainnet: 137, amoy: 80002)
        else if (chainId == 137 || chainId == 80002) {
            HostParams memory params = EvmHost(POLYGON_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(POLYGON_HOST).updateHostParams(params);
        }
        // Soneium (mainnet: 1868)
        else if (chainId == 1868) {
            HostParams memory params = EvmHost(SONEIUM_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(SONEIUM_HOST).updateHostParams(params);
        }
        // Unichain (mainnet: 1301)
        else if (chainId == 1301) {
            HostParams memory params = EvmHost(UNICHAIN_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(UNICHAIN_HOST).updateHostParams(params);
        }
        // Sei (mainnet: 1329, arctic testnet: 713715)
        else if (chainId == 1329 || chainId == 713715) {
            HostParams memory params = EvmHost(SEI_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            // params.handler = address(handler);
            EvmHost(SEI_HOST).updateHostParams(params);
        } else {
            revert("Unknown chain ID");
        }
    }
}
