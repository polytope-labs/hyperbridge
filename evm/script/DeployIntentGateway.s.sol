// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {IntentGatewayV2, Params} from "../src/apps/IntentGatewayV2.sol";
import {BaseScript} from "./BaseScript.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {SolverAccount} from "../src/utils/SolverAccount.sol";
import {VWAPOracle} from "../src/utils/VWAPOracle.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        IntentGatewayV2 intentGateway = new IntentGatewayV2{salt: salt}(admin);
        console.log("IntentGateway deployed at:", address(intentGateway));

        SolverAccount solverAccount = new SolverAccount{salt: salt}(address(intentGateway));
        console.log("SolverAccount deployed at:", address(solverAccount));
        VWAPOracle priceOracle = new VWAPOracle{salt: salt}(admin);
        console.log("VWAPOracle deployed at:", address(priceOracle));

        // Initialize price oracle with token decimals
        VWAPOracle.TokenDecimalsUpdate[] memory decimalsUpdates = new VWAPOracle.TokenDecimalsUpdate[](8);

        // Ethereum
        decimalsUpdates[0].sourceChain = bytes("EVM-1");
        decimalsUpdates[0].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[0].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48, // USDC
            decimals: 6
        });
        decimalsUpdates[0].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0xdAC17F958D2ee523a2206206994597C13D831ec7, // USDT
            decimals: 6
        });

        // Arbitrum
        decimalsUpdates[1].sourceChain = bytes("EVM-42161");
        decimalsUpdates[1].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[1].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0xaf88d065e77c8cC2239327C5EDb3A432268e5831, // USDC
            decimals: 6
        });
        decimalsUpdates[1].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9, // USDT
            decimals: 6
        });

        // Optimism
        decimalsUpdates[2].sourceChain = bytes("EVM-10");
        decimalsUpdates[2].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[2].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85, // USDC
            decimals: 6
        });
        decimalsUpdates[2].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0x94b008aA00579c1307B0EF2c499aD98a8ce58e58, // USDT
            decimals: 6
        });

        // Base
        decimalsUpdates[3].sourceChain = bytes("EVM-8453");
        decimalsUpdates[3].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[3].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913, // USDC
            decimals: 6
        });
        decimalsUpdates[3].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2, // USDT
            decimals: 6
        });

        // BSC
        decimalsUpdates[4].sourceChain = bytes("EVM-56");
        decimalsUpdates[4].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[4].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d, // USDC
            decimals: 18
        });
        decimalsUpdates[4].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0x55d398326f99059fF775485246999027B3197955, // USDT
            decimals: 18
        });

        // Gnosis
        decimalsUpdates[5].sourceChain = bytes("EVM-100");
        decimalsUpdates[5].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[5].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0xDDAfbb505ad214D7b80b1f830fcCc89B60fb7A83, // USDC (WXDAI)
            decimals: 6
        });
        decimalsUpdates[5].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0x4ECaBa5870353805a9F068101A40E0f32ed605C6, // USDT
            decimals: 6
        });

        // Polygon
        decimalsUpdates[6].sourceChain = bytes("EVM-137");
        decimalsUpdates[6].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[6].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359, // USDC
            decimals: 6
        });
        decimalsUpdates[6].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0xc2132D05D31c914a87C6611C10748AEb04B58e8F, // USDT
            decimals: 6
        });

        // Unichain
        decimalsUpdates[7].sourceChain = bytes("EVM-130");
        decimalsUpdates[7].tokens = new VWAPOracle.TokenDecimal[](2);
        decimalsUpdates[7].tokens[0] = VWAPOracle.TokenDecimal({
            token: 0x078D782b760474a361dDA0AF3839290b0EF57AD6, // USDC 
            decimals: 6
        });
        decimalsUpdates[7].tokens[1] = VWAPOracle.TokenDecimal({
            token: 0x9151434b16b9763660705744891fA906F660EcC5, // USDT 
            decimals: 6
        });

        priceOracle.init(HOST_ADDRESS, decimalsUpdates);
        console.log("VWAPOracle initialized with token decimals");

        intentGateway.setParams(Params({
            host: HOST_ADDRESS,
            dispatcher: config.get("CALL_DISPATCHER").toAddress(),
            solverSelection: config.get("7702").toBool(),
            surplusShareBps: 0,
            protocolFeeBps: 0,
            priceOracle: address(priceOracle)
        }));

        config.set("INTENT_GATEWAY", address(intentGateway));
        config.set("SOLVER_ACCOUNT", address(solverAccount));
        config.set("PRICE_ORACLE", address(priceOracle));
    }
}
