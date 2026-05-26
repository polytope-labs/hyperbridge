// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {GnosisUniswapV2Interface} from "../src/utils/uniswapv2/GnosisUniswapV2Wrapper.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        // The Gnosis wrapper is wrap-only: the native gas token (xDAI) is already a
        // dollar stable, so it just wraps xDAI -> WXDAI. No router/quoter/fee config needed.
        GnosisUniswapV2Interface wrapper = new GnosisUniswapV2Interface{salt: salt}();
        vm.stopBroadcast();
        console.log("GnosisUniswapV2Interface deployed at:", address(wrapper));
        // Persist the deployed wrapper address into the UNISWAP_V2 config field.
        config.set("UNISWAP_V2", address(wrapper));
    }
}
