// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../src/utils/SimplexPaymaster.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function deploy() internal override {
        address nativeOracleAddr = config.get("NATIVE_ORACLE").toAddress();
        uint256 markupBps = vm.envOr("MARKUP_BPS", uint256(200)); // default 2%
        address treasury = vm.envOr("TREASURY", admin); // default to owner

        SimplexPaymaster implementation = new SimplexPaymaster{salt: salt}();
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (AggregatorV3Interface(nativeOracleAddr), markupBps, treasury, admin)
        );
        ERC1967Proxy proxy = new ERC1967Proxy{salt: salt}(address(implementation), initData);
        SimplexPaymaster paymaster = SimplexPaymaster(address(proxy));

        // Stablecoin feeds on Ethereum and Base run a 24h heartbeat; a buffer over
        // the 24h default avoids transient StaleOraclePrice reverts on late pushes.
        uint256 maxOracleAge = vm.envOr("MAX_ORACLE_AGE", uint256(90_000));
        paymaster.setMaxOracleAge(maxOracleAge);

        console.log("SimplexPaymaster implementation deployed at:", address(implementation));
        console.log("SimplexPaymaster proxy deployed at:", address(paymaster));
        console.log("  nativeOracle:", nativeOracleAddr);
        console.log("  markupBps:", markupBps);
        console.log("  maxOracleAge:", maxOracleAge);
        console.log("  treasury:", treasury);
        console.log("  owner:", admin);

        address usdcToken = config.get("USDC_TOKEN").toAddress();
        address usdcOracle = config.get("USDC_ORACLE").toAddress();
        paymaster.registerToken(usdcToken, AggregatorV3Interface(usdcOracle));
        console.log("  Registered USDC:", usdcToken, "oracle:", usdcOracle);

        if (config.exists("USDT_TOKEN") && config.exists("USDT_ORACLE")) {
            address usdtToken = config.get("USDT_TOKEN").toAddress();
            address usdtOracle = config.get("USDT_ORACLE").toAddress();
            paymaster.registerToken(usdtToken, AggregatorV3Interface(usdtOracle));
            console.log("  Registered USDT:", usdtToken, "oracle:", usdtOracle);
        }

        config.set("SIMPLEX_PAYMASTER", address(paymaster));

        console.log("");
        console.log("=== IMPORTANT: Post-deployment steps ===");
        console.log("1. Fund the EntryPoint deposit for the paymaster:");
        console.log("   cast send <ENTRY_POINT> \"depositTo(address)\" ", address(paymaster), " --value 0.01ether");
        console.log("2. Transfer ownership to the multisig (two-step):");
        console.log("   cast send", address(paymaster), "\"transferOwnership(address)\" <MULTISIG>");
        console.log("   then accept from the multisig: acceptOwnership()");
    }
}
