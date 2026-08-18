// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {ERC4337Utils} from "@openzeppelin/contracts/account/utils/draft-ERC4337Utils.sol";

import {SimplexPaymaster, AggregatorV3Interface} from "../src/utils/SimplexPaymaster.sol";
import {SolverAccount} from "../src/apps/intentsv2/SolverAccount.sol";

/// @dev Freely mintable stand-in for a stablecoin without EIP-2612.
contract ProbeToken is ERC20 {
    constructor() ERC20("Probe USD", "pUSD") {}

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @notice Base Sepolia side deployment for probing PERMIT2 mode through live bundlers:
///         a mintable no-permit token priced off the USDC/USD feed, a SolverAccount to
///         delegate to, and a fully configured, deposited and staked SimplexPaymaster.
///         Usage: PRIVATE_KEY=... forge script script/SimplexPaymasterPermit2Probe.s.sol \
///                --rpc-url <base-sepolia> --broadcast
contract SimplexPaymasterPermit2ProbeScript is Script {
    address constant HOST = 0x9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1;
    address constant INTENT_GATEWAY_V2 = 0x6CF42FA9BecbC5b6a26884964956b113530f7cFA;
    address constant ETH_USD = 0x4aDC67696bA383F43DD60A9e78F2C97Fbbfc7cb1;
    address constant USDC_USD = 0xd30e2101a97dcbAeBCBC04F14C3f624E67A35165;

    function run() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(pk);
        vm.startBroadcast(pk);

        ProbeToken token = new ProbeToken();
        token.mint(deployer, 1_000e6);

        SolverAccount solverAccount = new SolverAccount(INTENT_GATEWAY_V2);

        address[] memory tokens = new address[](1);
        tokens[0] = address(token);
        AggregatorV3Interface[] memory oracles = new AggregatorV3Interface[](1);
        oracles[0] = AggregatorV3Interface(USDC_USD);

        SimplexPaymaster implementation = new SimplexPaymaster();
        bytes memory initData = abi.encodeCall(
            SimplexPaymaster.initialize,
            (
                HOST,
                SimplexPaymaster.Params({
                    nativeOracle: AggregatorV3Interface(ETH_USD),
                    markupBps: 200,
                    treasury: deployer,
                    maxOracleAge: 90_000,
                    swapSlippageBps: 200
                }),
                tokens,
                oracles
            )
        );
        SimplexPaymaster paymaster = SimplexPaymaster(payable(address(new ERC1967Proxy(address(implementation), initData))));

        ERC4337Utils.ENTRYPOINT_V08.depositTo{value: 0.05 ether}(address(paymaster));
        paymaster.addStake{value: 0.1 ether}(86_400);

        vm.stopBroadcast();

        console.log("token:", address(token));
        console.log("solverAccount:", address(solverAccount));
        console.log("paymaster implementation:", address(implementation));
        console.log("paymaster proxy:", address(paymaster));
    }
}
