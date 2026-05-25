// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {Config} from "forge-std/Config.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IERC20Metadata} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import {IUniswapV2Router02} from "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";

/// @notice For each mainnet network, forks it, reads the configured `UNISWAP_V2` wrapper and
/// `FEE_TOKEN` from `config.mainnet.toml`, and asserts that calling `swapETHForExactTokens`
/// through the `IUniswapV2Router02` interface (exactly as `EvmHost` does) delivers the fee
/// token. This validates that the configured wrapper is wired to a pool with real liquidity.
/// Polkadot Hub is excluded — it has no DEX wrapper.
contract ConfiguredWrapperForkTest is Test, Config {
    string internal constant CONFIG_FILE = "config.mainnet.toml";

    function _assertSwapWorks(string memory chain) internal {
        // Skipped on CI (no per-chain mainnet RPC aliases). Run locally with the
        // mainnet env sourced and RUN_WRAPPER_FORK_TESTS=true.
        if (!vm.envOr("RUN_WRAPPER_FORK_TESTS", false)) {
            vm.skip(true);
            return;
        }
        vm.createSelectFork(chain);
        _loadConfig(CONFIG_FILE, false);

        IUniswapV2Router02 router = IUniswapV2Router02(config.get("UNISWAP_V2").toAddress());
        address feeToken = config.get("FEE_TOKEN").toAddress();
        address weth = router.WETH();

        // ~1 unit of the fee token (e.g. 1 USDC / 1 USDT / 1 WXDAI).
        uint256 amountOut = 10 ** IERC20Metadata(feeToken).decimals();

        address[] memory path = new address[](2);
        path[0] = weth;
        path[1] = feeToken;

        uint256 budget = 100 ether; // generous amountInMaximum; the wrapper refunds the excess
        vm.deal(address(this), budget);

        uint256 balanceBefore = IERC20(feeToken).balanceOf(address(this));
        router.swapETHForExactTokens{value: budget}(amountOut, path, address(this), block.timestamp + 1 hours);
        uint256 received = IERC20(feeToken).balanceOf(address(this)) - balanceBefore;

        assertGe(received, amountOut, "configured uniswapV2 did not deliver the fee token");
    }

    function testSwapEthereum() public {
        _assertSwapWorks("ethereum");
    }

    function testSwapOptimism() public {
        _assertSwapWorks("optimism");
    }

    function testSwapPolygon() public {
        _assertSwapWorks("polygon");
    }

    function testSwapArbitrum() public {
        _assertSwapWorks("arbitrum");
    }

    function testSwapBase() public {
        _assertSwapWorks("base");
    }

    function testSwapBsc() public {
        _assertSwapWorks("bsc");
    }

    function testSwapGnosis() public {
        _assertSwapWorks("gnosis");
    }

    function testSwapSoneium() public {
        _assertSwapWorks("soneium");
    }

    // Required to receive ETH refunds from the wrappers.
    receive() external payable {}
}
