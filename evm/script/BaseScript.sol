// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {Config} from "forge-std/Config.sol";

abstract contract BaseScript is Script, Config {
    // ============= Environment Variables =============
    bytes32 internal privateKey = vm.envBytes32("PRIVATE_KEY");
    address internal admin = vm.envAddress("ADMIN");
    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));
    bytes internal consensusState = vm.envBytes("CONSENSUS_STATE");
    bytes32 internal sp1VerificationKey = vm.envBytes32("SP1_VERIFICATION_KEY");

    // ============= Config Variables =============
    address payable internal HOST_ADDRESS;

    function setUp() public {
        // Load config
        _loadConfig(vm.envString("CONFIG"), true);

    }

    /// @notice Deploy to a single chain (current network)
    /// @dev This is the default entry point for single-chain deployments
    function run() external {
        console.log("=================================");
        console.log("Single Chain Deployment");
        console.log("Chain ID:", block.chainid);
        HOST_ADDRESS = payable(config.get("HOST").toAddress());
        console.log("Host Address:", HOST_ADDRESS);
        console.log("=================================\n");

        vm.startBroadcast(uint256(privateKey));

        deploy();

        vm.stopBroadcast();

        console.log("\n=================================");
        console.log("Deployment Completed");
        console.log("=================================\n");
    }

    /// @notice Deploy to multiple chains using fork selection
    /// @dev Use with: forge script Script.s.sol --sig "run(string[])" "[chain1,chain2,chain3]" --multi --broadcast
    /// @param chains Array of chain names matching those in foundry.toml [rpc_endpoints]
    function run(string[] calldata chains) external {
        require(chains.length > 0, "At least one chain must be specified");

        console.log("=================================");
        console.log("Multi-Chain Deployment Started");
        console.log("Deploying to", chains.length, "chains");
        console.log("=================================\n");

        for (uint256 i = 0; i < chains.length; i++) {
            string memory chain = chains[i];

            console.log("\n=================================");
            console.log("Deploying to chain:", chain);
            console.log("=================================\n");

            deployToChain(chain);
        }

        console.log("\n=================================");
        console.log("Multi-Chain Deployment Completed");
        console.log("=================================\n");
    }

    /// @notice Deploy to a specific chain using fork selection
    /// @param chain The chain name matching foundry.toml [rpc_endpoints]
    function deployToChain(string memory chain) internal {
        // Create and select fork for the target chain
        vm.createSelectFork(chain);

        uint256 chainId = block.chainid;
        console.log("Chain ID:", chainId);

        // Reload config for this specific chain
        string memory configPath = vm.envString("CONFIG");
        _loadConfig(configPath, true);

        // Get chain-specific HOST address from config
        HOST_ADDRESS = payable(config.get("HOST").toAddress());
        console.log("Host address:", HOST_ADDRESS);

        // Start broadcasting transactions
        vm.startBroadcast(uint256(privateKey));

        // Call the child contract's deploy function
        deploy();

        vm.stopBroadcast();

        console.log("Deployment completed for chain:", chain, "\n");
    }

    /// @notice Abstract deploy function to be implemented by child contracts
    /// @dev This function should contain all deployment logic
    /// @dev Will be called within a broadcast context (vm.startBroadcast/stopBroadcast)
    function deploy() internal virtual;
}
