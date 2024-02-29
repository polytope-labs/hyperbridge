// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import "stringutils/strings.sol";

import "../src/modules/HandlerV1.sol";
import "../src/hosts/EvmHost.sol";
import "../src/modules/HostManager.sol";

import "../src/consensus/BeefyV1.sol";
import "../src/hosts/Ethereum.sol";
import "../src/hosts/Arbitrum.sol";
import "../src/hosts/Optimism.sol";
import "../src/hosts/Base.sol";

import {PingModule} from "../examples/PingModule.sol";
import {BscHost} from "../src/hosts/Bsc.sol";
import {PolygonHost} from "../src/hosts/Polygon.sol";
import {RococoVerifier} from "../src/consensus/verifiers/RococoVerifier.sol";
import {ZkBeefyV1} from "../src/consensus/ZkBeefy.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {GovernableToken} from "../src/modules/GovernableToken.sol";
import {StateMachine} from "ismp/StateMachine.sol";

contract DeployScript is Script {
    using strings for *;

    function run() external {
        address admin = vm.envAddress("ADMIN");
        uint256 paraId = vm.envUint("PARA_ID");
        string memory host = vm.envString("HOST");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        bytes32 salt = keccak256(bytes(vm.envString("VERSION")));

        vm.startBroadcast(uint256(privateKey));

        GovernableToken feeToken = new GovernableToken{salt: salt}(admin, "Hyper USD", "USD.h");
        // mint $1b to
        feeToken.mint(0x276b41950829E5A7B179ba03B758FaaE9A8d7C41, 1000000000 * 1e18, "");

        // consensus client
        //        RococoVerifier verifier = new RococoVerifier();
        //        ZkBeefyV1 consensusClient = new ZkBeefyV1{salt: salt}(paraId, verifier);
        BeefyV1 consensusClient = new BeefyV1{salt: salt}(paraId);

        // handler
        HandlerV1 handler = new HandlerV1{salt: salt}();

        // Host manager
        HostManager manager = new HostManager{salt: salt}(
            HostManagerParams({admin: admin, host: address(0), hyperbridge: StateMachine.kusama(paraId)})
        );

        // EvmHost
        HostParams memory params = HostParams({
            admin: admin,
            hostManager: address(manager),
            handler: address(handler),
            // 2hrs
            defaultTimeout: 2 * 60 * 60,
            // 21 days
            unStakingPeriod: 21 * (60 * 60 * 24),
            // for this test
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            lastUpdated: 0,
            consensusState: new bytes(0),
            baseGetRequestFee: 5 * 1e17, // $0.50
            perByteFee: 3 * 1e15, // $0.003/byte
            feeTokenAddress: address(feeToken),
            latestStateMachineHeight: 0
        });

        address hostAddress = initHost(host, params, salt);

        // set the host address on the host manager
        manager.setIsmpHost(hostAddress);
        feeToken.setIsmpHost(hostAddress);

        // deploy the ping module as well
        PingModule module = new PingModule{salt: salt}(admin);
        module.setIsmpHost(hostAddress);
        vm.stopBroadcast();
    }

    function initHost(string memory host, HostParams memory params, bytes32 salt) public returns (address) {
        if (Strings.equal(host, "sepolia") || host.toSlice().startsWith("eth".toSlice())) {
            EthereumHost h = new EthereumHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            ArbitrumHost h = new ArbitrumHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            OptimismHost h = new OptimismHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            BaseHost h = new BaseHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            BscHost h = new BscHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("polygon".toSlice())) {
            PolygonHost h = new PolygonHost{salt: salt}(params);
            return address(h);
        }

        revert("Unknown host");
    }
}
