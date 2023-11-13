// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";

import "../src/HandlerV1.sol";
import "../src/EvmHost.sol";
import "../src/modules/CrossChainGovernor.sol";

import "../src/beefy/BeefyV1.sol";
import "../src/hosts/Ethereum.sol";
import "../src/hosts/Arbitrum.sol";
import "../src/hosts/Optimism.sol";
import "../src/hosts/Base.sol";
import "../test/PingModule.sol";

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
        GovernorParams memory gParams = GovernorParams({admin: admin, host: address(0), paraId: paraId});
        CrossChainGovernor governor = new CrossChainGovernor{salt: salt}(
            gParams
        );
        // EvmHost
        HostParams memory params = HostParams({
            admin: admin,
            crosschainGovernor: address(governor),
            handler: address(handler),
            // 45 mins
            defaultTimeout: 45 * 60,
            // 21 days
            unStakingPeriod: 21 * (60 * 60 * 24),
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            lastUpdated: 0,
            consensusState: new bytes(0)
        });
        address hostAddress = initHost(host, params);
        // set the ismphost on the cross-chain governor
        governor.setIsmpHost(hostAddress);
        // deploy the ping module as well
        PingModule m = new PingModule{salt: salt}(hostAddress);
        vm.stopBroadcast();
    }

    function initHost(string memory host, HostParams memory params) public returns (address) {
        if (Strings.equal(host, "sepolia") || Strings.equal(host, "ethereum")) {
            EthereumHost host = new EthereumHost{salt: salt}(params);
            return address(host);
        } else if (Strings.equal(host, "arbitrum-sepolia")) {
            ArbitrumHost host = new ArbitrumHost{salt: salt}(params);
            return address(host);
        } else if (Strings.equal(host, "optimism-sepolia")) {
            OptimismHost host = new OptimismHost{salt: salt}(params);
            return address(host);
        } else if (Strings.equal(host, "base-sepolia")) {
            BaseHost host = new BaseHost{salt: salt}(params);
            return address(host);
        }

        revert("unknown host");
    }
}
