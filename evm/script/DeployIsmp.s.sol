// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "stringutils/strings.sol";


import {SP1Verifier} from "@sp1-contracts/v6.1.0/SP1VerifierGroth16.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {UniV4UniswapV2Wrapper} from "../src/utils/uniswapv2/UniV4UniswapV2Wrapper.sol";
import {IConsensusV2} from "@hyperbridge/core/interfaces/IConsensusV2.sol";
import {StateMachine} from "@hyperbridge/core/libraries/StateMachine.sol";

import "../src/core/HandlerV2.sol";
import "../src/core/EvmHost.sol";
import "../src/core/HostManager.sol";
import "../src/consensus/EcdsaBeefy.sol";
import "../src/consensus/ConsensusRouter.sol";

import {TestnetHost} from "../src/core/TestnetHost.sol";
import {BandwidthManager} from "../src/apps/BandwidthManager.sol";
import {HyperFungibleTokenImpl} from "../src/utils/HyperFungibleTokenImpl.sol";
import {TokenFaucet} from "../src/utils/TokenFaucet.sol";
import {SP1Beefy} from "../src/consensus/SP1Beefy.sol";
import {EcdsaBeefy} from "../src/consensus/EcdsaBeefy.sol";
import {ConsensusRouter} from "../src/consensus/ConsensusRouter.sol";

import {FeeToken} from "../tests/foundry/FeeToken.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {BaseScript} from "./BaseScript.sol";


bytes32 constant MINTER_ROLE = keccak256("MINTER ROLE");
bytes32 constant BURNER_ROLE = keccak256("BURNER ROLE");

contract DeployScript is BaseScript {
    using strings for *;

    uint256 private paraId = vm.envUint("PARA_ID");

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        uint256 decimals;
        address uniswapV2;
        address consensusClient;
        address feeToken;
        bytes memory hyperbridge;
        TokenFaucet faucet;
        HyperFungibleTokenImpl feeTokenInstance;

        bool isMainnet = config.get("is_mainnet").toBool();

        // Deploy SP1 ZK consensus client
        SP1Verifier verifier = new SP1Verifier{salt: salt}();
        SP1Beefy sp1Beefy = new SP1Beefy{salt: salt}(verifier, sp1VerificationKey);
        // Deploy EcdsaBeefy naive consensus client
        EcdsaBeefy ecdsaBeefy = new EcdsaBeefy{salt: salt}();
        // Deploy ConsensusRouter wrapping both consensus clients
        ConsensusRouter consensusRouter = new ConsensusRouter{salt: salt}(
            IConsensusV2(address(sp1Beefy)), IConsensusV2(address(ecdsaBeefy))
        );
        consensusClient = address(consensusRouter);

        if (isMainnet) {
            // use feeToken configured in environment variables
            address uniswap = config.get("UNISWAP_V2").toAddress();
            // if existing univ2 address isn't available, deploy univ4 wrapper
            if (uniswap == address(0)) {
                address universalRouter = config.get("UNIVERSAL_ROUTER").toAddress();
                if (universalRouter != address(0)) {
                    UniV4UniswapV2Wrapper wrapper = new UniV4UniswapV2Wrapper{salt: salt}(admin);
                    wrapper.init(
                        UniV4UniswapV2Wrapper.Params({
                            universalRouter: universalRouter,
                            quoter: config.get("V4_QUOTER").toAddress(),
                            WETH: config.get("WETH").toAddress(),
                            defaultFee: uint24(config.get("DEFAULT_FEE").toUint256()),
                            defaultTickSpacing: int24(config.get("DEFAULT_TICK_SPACING").toInt256())
                        })
                    );
                    uniswapV2 = address(wrapper);
                }
            } else {
                uniswapV2 = uniswap;
            }

            feeToken = config.get("FEE_TOKEN").toAddress();
            // Allow the fee token's decimals to be set explicitly in config. Needed on chains
            // where the fee token is a runtime asset precompile (e.g. Polkadot Hub USDC) whose
            // decimals() cannot be executed in forge's local fork. Falls back to an on-chain
            // read when not configured.
            if (config.exists("FEE_TOKEN_DECIMALS")) {
                decimals = config.get("FEE_TOKEN_DECIMALS").toUint256();
            } else {
                decimals = IERC20Metadata(feeToken).decimals();
            }
            hyperbridge = StateMachine.polkadot(paraId);
        } else {
            // Deploy our own feetoken contract & faucet
            faucet = new TokenFaucet{salt: salt}();
            feeTokenInstance = new HyperFungibleTokenImpl{salt: salt}(admin, "Hyper USD", "USD.h");
            // Grant minter role to faucet so it can mint tokens
            feeTokenInstance.grantMinterRole(address(faucet));
            feeToken = address(feeTokenInstance);
            hyperbridge = StateMachine.kusama(paraId);
            decimals = 18;
        }

        // handler
        HandlerV2 handler = new HandlerV2{salt: salt}();
        // Host manager
        HostManager manager = new HostManager{salt: salt}(HostManagerParams({admin: admin, host: address(0)}));
        uint256[] memory stateMachines = new uint256[](1);
        stateMachines[0] = paraId;

        // EvmHost
        HostParams memory params = HostParams({
            uniswapV2: uniswapV2,
            admin: admin,
            hostManager: address(manager),
            handler: address(handler),
            unStakingPeriod: 21 * (60 * 60 * 24),
            challengePeriod: 0,
            consensusClient: address(consensusClient),
            hyperbridge: hyperbridge,
            feeToken: feeToken,
            stateMachines: stateMachines
        });

        EvmHost host = isMainnet
            ? new EvmHost{salt: salt}(admin)
            : EvmHost(payable(address(new TestnetHost{salt: salt}(admin))));
        host.initialize(params);
        // set the host address on the host manager
        manager.setIsmpHost(address(host));

        // Set the consensus state
        EvmHost(payable(address(host)))
            .setConsensusState(
                consensusState,
                StateMachineHeight({stateMachineId: paraId, height: 1}),
                StateCommitment({timestamp: block.timestamp, overlayRoot: bytes32(0), stateRoot: bytes32(0)})
            );

        // ============= Deploy applications =============
        CallDispatcher callDispatcher = new CallDispatcher{salt: salt}();
        BandwidthManager bandwidthManager = new BandwidthManager{salt: salt}(admin);
        bandwidthManager.setHost(address(host));
        
        vm.stopBroadcast();

        // ============= Write addresses to config =============
        if (!isMainnet) {
            config.set("TOKEN_FAUCET", address(faucet));
            config.set("FEE_TOKEN", feeToken);
        }
        config.set("HOST", address(host));
        config.set("ECDSA_BEEFY", address(ecdsaBeefy));
        config.set("SP1_BEEFY", address(sp1Beefy));
        config.set("HANDLER_V2", address(handler));
        config.set("CONSENSUS_ROUTER", address(consensusRouter));
        config.set("CALL_DISPATCHER", address(callDispatcher));
        config.set("BANDWIDTH_MANAGER", address(bandwidthManager));
    }
}
