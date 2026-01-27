// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {UniV4UniswapV2Wrapper} from "../src/utils/uniswapv2/UniV4UniswapV2Wrapper.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        address universalRouter = config.get("UNIVERSAL_ROUTER").toAddress();
        address quoter = config.get("V4_QUOTER").toAddress();
        uint24 defaultFee = uint24(config.get("DEFAULT_FEE").toUint256());
        int24 defaultTickSpacing = int24(config.get("DEFAULT_TICK_SPACING").toInt256());
        address weth = config.get("WETH").toAddress();

        UniV4UniswapV2Wrapper wrapper = new UniV4UniswapV2Wrapper{salt: salt}(admin);
        console.log("UniV4UniswapV2Wrapper deployed at:", address(wrapper));

        wrapper.init(
            UniV4UniswapV2Wrapper.Params({
                universalRouter: universalRouter,
                quoter: quoter,
                WETH: weth,
                defaultFee: defaultFee,
                defaultTickSpacing: defaultTickSpacing
            })
        );
        console.log("UniV4UniswapV2Wrapper initialized");
    }
}
