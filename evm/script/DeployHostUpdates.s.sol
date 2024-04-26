// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import "stringutils/strings.sol";

import {EvmHost, HostParams} from "../src/hosts/EvmHost.sol";
import {RococoVerifier} from "../src/consensus/verifiers/RococoVerifier.sol";
import {ZkBeefyV1} from "../src/consensus/ZkBeefy.sol";

contract DeployScript is Script {
    using strings for *;

    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));

    address public SEPOLIA_HOST = vm.envAddress("SEPOLIA_HOST");
    address public ARB_SEPOLIA_HOST = vm.envAddress("ARB_SEPOLIA_HOST");
    address public OP_SEPOLIA_HOST = vm.envAddress("OP_SEPOLIA_HOST");
    address public BASE_SEPOLIA_HOST = vm.envAddress("BASE_SEPOLIA_HOST");
    address public BSC_TESTNET_HOST = vm.envAddress("BSC_TESTNET_HOST");

    uint256 public paraId = vm.envUint("PARA_ID");
    string private host = vm.envString("HOST");
    bytes32 private privateKey = vm.envBytes32("PRIVATE_KEY");

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        if (Strings.equal(host, "sepolia") || host.toSlice().startsWith("eth".toSlice())) {
            HostParams memory params = EvmHost(SEPOLIA_HOST).hostParams();
            params.consensusUpdateTimestamp = block.timestamp;
            EvmHost(SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            HostParams memory params = EvmHost(ARB_SEPOLIA_HOST).hostParams();
            params.consensusUpdateTimestamp = block.timestamp;
            EvmHost(ARB_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            HostParams memory params = EvmHost(OP_SEPOLIA_HOST).hostParams();
            params.consensusUpdateTimestamp = block.timestamp;
            EvmHost(OP_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            HostParams memory params = EvmHost(BASE_SEPOLIA_HOST).hostParams();
            params.consensusUpdateTimestamp = block.timestamp;
            EvmHost(BASE_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            HostParams memory params = EvmHost(BSC_TESTNET_HOST).hostParams();
            params.consensusUpdateTimestamp = block.timestamp;
            EvmHost(BSC_TESTNET_HOST).setHostParamsAdmin(params);
        } else {
            revert("Unknown Host");
        }
    }
}
