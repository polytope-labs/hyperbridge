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

import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";
import {TokenGateway, Asset, InitParams} from "../src/modules/TokenGateway.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";

import {PingModule} from "../examples/PingModule.sol";
import {BscHost} from "../src/hosts/Bsc.sol";
import {PolygonHost} from "../src/hosts/Polygon.sol";
import {RococoVerifier} from "../src/consensus/verifiers/RococoVerifier.sol";
import {ZkBeefyV1} from "../src/consensus/ZkBeefy.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {GovernableToken} from "../src/modules/GovernableToken.sol";
import {StateMachine} from "ismp/StateMachine.sol";
import {FeeToken} from "../test/FeeToken.sol";

bytes32 constant MINTER_ROLE = keccak256("MINTER ROLE");
bytes32 constant BURNER_ROLE = keccak256("BURNER ROLE");

contract DeployScript is Script {
    using strings for *;

    address private admin = vm.envAddress("ADMIN");
    uint256 private paraId = vm.envUint("PARA_ID");
    string private host = vm.envString("HOST");
    bytes32 private privateKey = vm.envBytes32("PRIVATE_KEY");
    bytes32 private salt = keccak256(bytes(vm.envString("VERSION")));

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        ERC6160Ext20 feeToken = new ERC6160Ext20{salt: salt}(admin, "Hyper USD", "USD.h");
        // mint $1b to
        feeToken.mint(0x276b41950829E5A7B179ba03B758FaaE9A8d7C41, 1000000000 * 1e18, "");

        // consensus client
        RococoVerifier verifier = new RococoVerifier();
        ZkBeefyV1 consensusClient = new ZkBeefyV1{salt: salt}(paraId, verifier);

        // handler
        HandlerV1 handler = new HandlerV1{salt: salt}();

        // Host manager
        HostManager manager = new HostManager{salt: salt}(HostManagerParams({admin: admin, host: address(0)}));

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
            latestStateMachineHeight: 0,
            hyperbridge: StateMachine.kusama(paraId)
        });

        address hostAddress = initHost(params);
        // set the host address on the host manager
        manager.setIsmpHost(hostAddress);

        // deploy the ping module as well
        PingModule module = new PingModule{salt: salt}(admin);
        module.setIsmpHost(hostAddress);

        deployGateway(feeToken, hostAddress);

        vm.stopBroadcast();
    }

    function initHost(HostParams memory params) public returns (address) {
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

    function deployGateway(ERC6160Ext20 feeToken, address hostAddress) public {
        // deploy token gateway
        TokenGateway gateway = new TokenGateway{salt: salt}(admin);
        feeToken.grantRole(MINTER_ROLE, address(gateway));
        feeToken.grantRole(BURNER_ROLE, address(gateway));

        // and token faucet
        TokenFaucet faucet = new TokenFaucet{salt: salt}(address(feeToken));
        feeToken.grantRole(MINTER_ROLE, address(faucet));

        Asset[] memory assets = new Asset[](1);
        assets[0] = Asset({
            localIdentifier: keccak256("USD.h"),
            foreignIdentifier: keccak256("USD.h"),
            erc20: address(0),
            erc6160: address(feeToken)
        });

        // initialize gateway
        gateway.init(
            InitParams({
                hyperbridge: StateMachine.kusama(paraId),
                host: hostAddress,
                uniswapV2Router: address(1),
                protocolFeePercentage: 100, // 0.1
                relayerFeePercentage: 300, // 0.3
                assets: assets
            })
        );
    }
}
