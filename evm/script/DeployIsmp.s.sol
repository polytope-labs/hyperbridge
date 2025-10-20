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
import "../src/hosts/Sei.sol";

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
import {IntentGateway, Params} from "../src/modules/IntentGateway.sol";
import {UniV3UniswapV2Wrapper} from "../src/modules/UniV3UniswapV2Wrapper.sol";

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

        bool isMainnet = config.get("is_mainnet").toBool();
        if (isMainnet) {
            // deploy zk connsensus client
            SP1Verifier verifier = new SP1Verifier{salt: salt}();
            SP1Beefy consensusClientInstance = new SP1Beefy{salt: salt}(verifier);
            consensusClient = address(consensusClientInstance);
            // use feeToken configured in environment variables
            address uniswap = config.get("UNISWAP_V2").toAddress();
            // if existing univ2 address isn't available, deploy univ3 wrapper
            if (uniswap == address(0)) {
                address WETH = config.get("WETH").toAddress();
                address SWAP_ROUTER = config.get("SWAP_ROUTER").toAddress();
                address QUOTER = config.get("QUOTER").toAddress();

                UniV3UniswapV2Wrapper wrapper = new UniV3UniswapV2Wrapper{salt: salt}(admin);
                wrapper.init(UniV3UniswapV2Wrapper.Params({
                    WETH: WETH,
                    swapRouter: SWAP_ROUTER,
                    quoter: QUOTER
                }));
                uniswapV2 = address(wrapper);
            } else {
                uniswapV2 = uniswap;
            }

            feeToken = config.get("FEE_TOKEN").toAddress();
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
            defaultPerByteFee: 3 * (10 ** (decimals - 3)), // $0.0003/byte
            stateCommitmentFee: 1 * (10 ** decimals), // $1
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

        // ======= Deploy applications =============
        CallDispatcher callDispatcher = new CallDispatcher{salt: salt}();

        // token gateway
        TokenGateway tokenGateway = new TokenGateway{salt: salt}(admin);
        AssetMetadata[] memory assets = new AssetMetadata[](0);
        tokenGateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({host: hostAddress, dispatcher: address(callDispatcher)}),
                assets: assets
            })
        );

        IntentGateway intentGateway = new IntentGateway{salt: salt}(admin);
        intentGateway.setParams(Params({host: hostAddress, dispatcher: address(callDispatcher)}));

        if (!isMainnet) {
            PingModule ping = new PingModule{salt: salt}(admin);
            ping.setIsmpHost(hostAddress, address(faucet));
            config.set("PING", address(ping));
            config.set("TOKEN_FAUCET", address(faucet));
        }

        config.set("HOST", hostAddress);
        config.set("CALL_DISPATCHER", address(callDispatcher));
        config.set("TOKEN_GATEWAY", address(tokenGateway));
        config.set("INTENT_GATEWAY", address(intentGateway));

        vm.stopBroadcast();
    }

    function initHost(HostParams memory params) public returns (address) {
        uint256 chainId = block.chainid;

        // Ethereum (mainnet: 1, sepolia: 11155111)
        if (chainId == 1 || chainId == 11155111) {
            EthereumHost h = new EthereumHost{salt: salt}(params);
            return address(h);
        }
        // Arbitrum (mainnet: 42161, sepolia: 421614)
        else if (chainId == 42161 || chainId == 421614) {
            ArbitrumHost h = new ArbitrumHost{salt: salt}(params);
            return address(h);
        }
        // Optimism (mainnet: 10, sepolia: 11155420)
        else if (chainId == 10 || chainId == 11155420) {
            OptimismHost h = new OptimismHost{salt: salt}(params);
            return address(h);
        }
        // Base (mainnet: 8453, sepolia: 84532)
        else if (chainId == 8453 || chainId == 84532) {
            BaseHost h = new BaseHost{salt: salt}(params);
            return address(h);
        }
        // BSC (mainnet: 56, testnet: 97)
        else if (chainId == 56 || chainId == 97) {
            BscHost h = new BscHost{salt: salt}(params);
            return address(h);
        }
        // Polygon (mainnet: 137, amoy: 80002)
        else if (chainId == 137 || chainId == 80002) {
            PolygonHost h = new PolygonHost{salt: salt}(params);
            return address(h);
        }
        // Gnosis (mainnet: 100, chiado: 10200)
        else if (chainId == 100 || chainId == 10200) {
            GnosisHost h = new GnosisHost{salt: salt}(params);
            return address(h);
        }
        // Soneium (mainnet: 1868)
        else if (chainId == 1868) {
            SoneiumHost h = new SoneiumHost{salt: salt}(params);
            return address(h);
        }
        // Unichain (mainnet: 1301)
        else if (chainId == 1301) {
            UnichainHost h = new UnichainHost{salt: salt}(params);
            return address(h);
        }
        // Sei (mainnet: 1329, arctic testnet: 1328)
        else if (chainId == 1329 || chainId == 1328) {
            SeiHost h = new SeiHost{salt: salt}(params);
            return address(h);
        }

        revert("Unknown chain ID");
    }
}
