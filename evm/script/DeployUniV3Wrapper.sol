// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {UniV3UniswapV2Wrapper} from "../src/utils/uniswapv2/UniV3UniswapV2Wrapper.sol";
import {BaseScript} from "./BaseScript.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {DispatchPost, DispatchGet, IDispatcher, PostRequest} from "@hyperbridge/core/interfaces/IDispatcher.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        address swapRouter = config.get("SWAP_ROUTER").toAddress();
        address quoter = config.get("QUOTER").toAddress();
        uint24 maxFee = uint24(config.get("MAX_FEE").toUint256());
        address uniswapV2 = IDispatcher(HOST_ADDRESS).uniswapV2Router();

        UniV3UniswapV2Wrapper wrapper = new UniV3UniswapV2Wrapper{salt: salt}(admin);
        console.log("UniV3UniswapV2Wrapper deployed at:", address(wrapper));

        wrapper.init(
            UniV3UniswapV2Wrapper.Params({
                WETH: IUniswapV2Router02(uniswapV2).WETH(), swapRouter: swapRouter, quoter: quoter, maxFee: maxFee
            })
        );
        console.log("UniV3UniswapV2Wrapper initialized");
    }
}
