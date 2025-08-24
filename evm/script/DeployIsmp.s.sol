// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import "../src/modules/HandlerV1.sol";
import "../src/hosts/EvmHost.sol";
import "../src/modules/HostManager.sol";

import "../src/consensus/BeefyV1.sol";
import "../src/hosts/Ethereum.sol";
import "../src/hosts/Arbitrum.sol";
import "../src/hosts/Optimism.sol";
import "../src/hosts/Base.sol";
import "../src/hosts/Gnosis.sol";
import "../src/hosts/Soneium.sol";
import "../src/hosts/Unichain.sol";

import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {TokenGateway, Asset, TokenGatewayParamsExt, TokenGatewayParams, AssetMetadata} from "../src/modules/TokenGateway.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";

import {PingModule} from "../examples/PingModule.sol";
import {BscHost} from "../src/hosts/Bsc.sol";
import {PolygonHost} from "../src/hosts/Polygon.sol";

import {SP1Verifier} from "@sp1-contracts/v4.0.0-rc.3/SP1VerifierGroth16.sol";
import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";
import {BeefyV1} from "../src/consensus/BeefyV1.sol";
import {StateMachine} from "@polytope-labs/ismp-solidity/StateMachine.sol";
import {FeeToken} from "../test/FeeToken.sol";
import {CallDispatcher} from "../src/modules/CallDispatcher.sol";
import {BaseScript} from "./BaseScript.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";

bytes32 constant MINTER_ROLE = keccak256("MINTER ROLE");
bytes32 constant BURNER_ROLE = keccak256("BURNER ROLE");

contract DeployScript is BaseScript {
    using strings for *;

    address private pingDispatcher = vm.envAddress("DISPATCHER");
    uint256 private paraId = vm.envUint("PARA_ID");

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        uint256 decimals;
        address uniswapV2;
        address consensusClient;
        address feeToken;
        bytes memory hyperbridge;
        TokenFaucet faucet;
        bool isMainnet = vm.envBool("MAINNET");

        if (isMainnet) {
            // deploy zk connsensus client
            SP1Verifier verifier = new SP1Verifier{salt: salt}();
            SP1Beefy consensusClientInstance = new SP1Beefy{salt: salt}(verifier);
            consensusClient = address(consensusClientInstance);
            // use feeToken configured in environment variables
            uniswapV2 = vm.envAddress(string.concat(host, "_UNISWAP_V2"));
            feeToken = vm.envAddress(string.concat(host, "_FEE_TOKEN"));
            decimals = IERC20Metadata(feeToken).decimals();
            hyperbridge = StateMachine.polkadot(paraId);
        } else {
            // deploy naive consensus client
            BeefyV1 consensusClientInstance = new BeefyV1{salt: salt}();
            consensusClient = address(consensusClientInstance);

            // Deploy our own feetoken contract & faucet
            ERC6160Ext20 feeTokenInstance = new ERC6160Ext20{salt: salt}(admin, "Hyper USD", "USD.h");
            faucet = new TokenFaucet{salt: salt}();
            feeTokenInstance.grantRole(feeTokenInstance.getMinterRole(), address(faucet));
            feeToken = address(feeTokenInstance);
            hyperbridge = StateMachine.kusama(paraId);
            decimals = 18;
        }

        // handler
        HandlerV1 handler = new HandlerV1{salt: salt}();

        // Host manager
        HostManager manager = new HostManager{salt: salt}(HostManagerParams({admin: admin, host: address(0)}));
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = paraId;

        // EvmHost
        PerByteFee[] memory perByteFees = new PerByteFee[](0);
        HostParams memory params = HostParams({
            uniswapV2: uniswapV2,
            perByteFees: perByteFees,
            admin: admin,
            hostManager: address(manager),
            handler: address(handler),
            defaultTimeout: 2 * 60 * 60,
            unStakingPeriod: 21 * (60 * 60 * 24),
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            defaultPerByteFee: 3 * (10 ** (decimals - 2)), // $0.003/byte
            stateCommitmentFee: 10 * (10 ** decimals), // $10
            hyperbridge: hyperbridge,
            feeToken: feeToken,
            stateMachines: stateMachines
        });

        address hostAddress = initHost(params);
        // set the host address on the host manager
        manager.setIsmpHost(hostAddress);

        // Set the consensus state
        EvmHost(payable(hostAddress)).setConsensusState(
            consensusState,
            StateMachineHeight({stateMachineId: paraId, height: 1}),
            StateCommitment({timestamp: block.timestamp, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
        );

        if (!isMainnet) {
            // deploy token gateway
            TokenGateway gateway = new TokenGateway{salt: salt}(admin);
            AssetMetadata[] memory assets = new AssetMetadata[](0);
            gateway.init(
                TokenGatewayParamsExt({
                    params: TokenGatewayParams({host: hostAddress, dispatcher: address(0)}),
                    assets: assets
                })
            );

            PingModule ping = new PingModule{salt: salt}(admin);
            ping.setIsmpHost(hostAddress, address(faucet));
        }

        vm.stopBroadcast();
    }

    function initHost(HostParams memory params) public returns (address) {
        if (host.toSlice().startsWith("ethereum".toSlice())) {
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
        } else if (host.toSlice().startsWith("gnosis".toSlice())) {
            GnosisHost h = new GnosisHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("soneium".toSlice())) {
            SoneiumHost h = new SoneiumHost{salt: salt}(params);
            return address(h);
        } else if (host.toSlice().startsWith("unichain".toSlice())) {
            UnichainHost h = new UnichainHost{salt: salt}(params);
            return address(h);
        }

        revert("Unknown host");
    }
}
