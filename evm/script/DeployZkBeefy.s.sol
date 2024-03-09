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

    address public SEPOLIA_HOST = 0xe4226c474A6f4BF285eA80c2f01c0942B04323e5;
    address public ARB_SEPOLIA_HOST = 0x56101AD00677488B3576C85e9e75d4F0a08BD627;
    address public OP_SEPOLIA_HOST = 0x39f3D7a7783653a04e2970e35e5f32F0e720daeB;
    address public BASE_SEPOLIA_HOST = 0x1D14e30e440B8DBA9765108eC291B7b66F98Fd09;
    address public BSC_TESTNET_HOST = 0x4e5bbdd9fE89F54157DDb64b21eD4D1CA1CDf9a6;

    uint256 public paraId = vm.envUint("PARA_ID");
    string private host = vm.envString("HOST");
    bytes32 private privateKey = vm.envBytes32("PRIVATE_KEY");

    function run() external {
        vm.startBroadcast(uint256(privateKey));
        // consensus client
        RococoVerifier verifier = new RococoVerifier{salt: salt}();
        ZkBeefyV1 consensusClient = new ZkBeefyV1{salt: salt}(paraId, verifier);

        if (Strings.equal(host, "sepolia") || host.toSlice().startsWith("eth".toSlice())) {
            HostParams memory params = EvmHost(SEPOLIA_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            EvmHost(SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            HostParams memory params = EvmHost(ARB_SEPOLIA_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            EvmHost(ARB_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            HostParams memory params = EvmHost(OP_SEPOLIA_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            EvmHost(OP_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            HostParams memory params = EvmHost(BASE_SEPOLIA_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            EvmHost(BASE_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            HostParams memory params = EvmHost(BSC_TESTNET_HOST).hostParams();
            params.consensusClient = address(consensusClient);
            EvmHost(BSC_TESTNET_HOST).setHostParamsAdmin(params);
        } else {
            revert("Unknown Host");
        }
    }
}
