// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";

import {UniV3UniswapV2Wrapper} from "../src/modules/UniV3UniswapV2Wrapper.sol";
import {BaseScript} from "./BaseScript.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import {DispatchPost, DispatchGet, IDispatcher, PostRequest} from "@polytope-labs/ismp-solidity-v1/IDispatcher.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        address hostAddr = vm.envAddress(string.concat(host, "_HOST"));
        address swapRouter = vm.envAddress(string.concat(host, "_SWAP_ROUTER"));
        address quoter = vm.envAddress(string.concat(host, "_QUOTER"));
        address uniswapV2 = IDispatcher(hostAddr).uniswapV2Router();

        UniV3UniswapV2Wrapper wrapper = new UniV3UniswapV2Wrapper{salt: salt}(admin);
        wrapper.init(UniV3UniswapV2Wrapper.Params({
            WETH: IUniswapV2Router02(uniswapV2).WETH(),
            swapRouter: swapRouter,
            quoter: quoter
        }));
        vm.stopBroadcast();
    }
}
