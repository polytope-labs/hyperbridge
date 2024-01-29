// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";

import "../src/HandlerV1.sol";
import "../src/EvmHost.sol";
import "../src/modules/HostManager.sol";

import "../src/beefy/BeefyV1.sol";
import "../src/hosts/Ethereum.sol";
import "../src/hosts/Arbitrum.sol";
import "../src/hosts/Optimism.sol";
import "../src/hosts/Base.sol";
import "../examples/PingModule.sol";
import {BscHost} from "../src/hosts/Bsc.sol";
import {PolygonHost} from "../src/hosts/Polygon.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes("gargantua-v0.0.1"));

    function run() external {
        address admin = vm.envAddress("ADMIN");
        uint256 paraId = vm.envUint("PARA_ID");
        string memory host = vm.envString("HOST");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        vm.startBroadcast(uint256(privateKey));

        // consensus client
        BeefyV1 consensusClient = new BeefyV1{salt: salt}(paraId);
        // handler
        HandlerV1 handler = new HandlerV1{salt: salt}();
        // cross-chain governor
        HostManagerParams memory gParams = HostManagerParams({admin: admin, host: address(0), paraId: paraId});
        HostManager governor = new HostManager{salt: salt}(gParams);
        // EvmHost
        HostParams memory params = HostParams({
            admin: admin,
            hostManager: address(governor),
            handler: address(handler),
            // 45 mins
            defaultTimeout: 45 * 60,
            // 21 days
            unStakingPeriod: 21 * (60 * 60 * 24),
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            lastUpdated: 0,
            consensusState: new bytes(0),
            baseGetRequestFee: 0,
            perByteFee: 0,
            feeTokenAddress: address(0),
            latestStateMachineHeight: 0
        });
        address hostAddress = initHost(host, params);
        // set the ismphost on the cross-chain governor
        governor.setIsmpHost(hostAddress);
        // deploy the ping module as well
        new PingModule{salt: salt}(hostAddress);
        vm.stopBroadcast();
    }

    function initHost(string memory host, HostParams memory params) public returns (address) {
        if (Strings.equal(host, "sepolia") || Strings.equal(host, "ethereum")) {
            EthereumHost h = new EthereumHost{salt: salt}(params);
            return address(h);
        } else if (Strings.equal(host, "arbitrum")) {
            ArbitrumHost h = new ArbitrumHost{salt: salt}(params);
            return address(h);
        } else if (Strings.equal(host, "optimism")) {
            OptimismHost h = new OptimismHost{salt: salt}(params);
            return address(h);
        } else if (Strings.equal(host, "base")) {
            BaseHost h = new BaseHost{salt: salt}(params);
            return address(h);
        } else if (Strings.equal(host, "bsc")) {
            BscHost h = new BscHost{salt: salt}(params);
            return address(h);
        } else if (Strings.equal(host, "polygon")) {
            PolygonHost h = new PolygonHost{salt: salt}(params);
            return address(h);
        }
    }
}
