// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import "stringutils/strings.sol";

import {EvmHost, HostParams} from "../contracts/hosts/EvmHost.sol";
import {BeefyV1} from "../contracts/consensus/BeefyV1.sol";

contract DeployScript is Script {
    using strings for *;

    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));

    address payable public SEPOLIA_HOST = payable(vm.envAddress("SEPOLIA_HOST"));
    address payable public ARB_SEPOLIA_HOST = payable(vm.envAddress("ARB_SEPOLIA_HOST"));
    address payable public OP_SEPOLIA_HOST = payable(vm.envAddress("OP_SEPOLIA_HOST"));
    address payable public BASE_SEPOLIA_HOST = payable(vm.envAddress("BASE_SEPOLIA_HOST"));
    address payable public BSC_TESTNET_HOST = payable(vm.envAddress("BSC_TESTNET_HOST"));

    uint256 public paraId = vm.envUint("PARA_ID");
    string private host = vm.envString("HOST");
    bytes32 private privateKey = vm.envBytes32("PRIVATE_KEY");

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        if (Strings.equal(host, "sepolia") || host.toSlice().startsWith("eth".toSlice())) {
            HostParams memory params = EvmHost(SEPOLIA_HOST).hostParams();
            EvmHost(SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            HostParams memory params = EvmHost(ARB_SEPOLIA_HOST).hostParams();
            EvmHost(ARB_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            HostParams memory params = EvmHost(OP_SEPOLIA_HOST).hostParams();
            EvmHost(OP_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            HostParams memory params = EvmHost(BASE_SEPOLIA_HOST).hostParams();
            EvmHost(BASE_SEPOLIA_HOST).setHostParamsAdmin(params);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            HostParams memory params = EvmHost(BSC_TESTNET_HOST).hostParams();
            EvmHost(BSC_TESTNET_HOST).setHostParamsAdmin(params);
        } else {
            revert("Unknown Host");
        }
    }
}
