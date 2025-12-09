// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {UniV4UniswapV2Wrapper} from "../src/modules/UniV4UniswapV2Wrapper.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        address universalRouter = config.get("UNIVERSAL_ROUTER").toAddress();
        address quoter = config.get("V4_QUOTER").toAddress();
        uint24 defaultFee = uint24(config.getUint("DEFAULT_FEE"));
        int24 defaultTickSpacing = int24(int256(config.getUint("DEFAULT_TICK_SPACING")));

        UniV4UniswapV2Wrapper wrapper = new UniV4UniswapV2Wrapper{salt: salt}(admin);
        wrapper.init(UniV4UniswapV2Wrapper.Params({
            universalRouter: universalRouter,
            quoter: quoter,
            defaultFee: defaultFee,
            defaultTickSpacing: defaultTickSpacing
        }));
        vm.stopBroadcast();
    }
}

