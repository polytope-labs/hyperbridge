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

    address public SEPOLIA_HOST = 0x9DF353352b469782AB1B0F2CbBFEC41bF1FDbDb3;
    address public ARB_SEPOLIA_HOST = 0x424e6971EB1C693cf4296d4bdb42aa0F32a0dd9e;
    address public OP_SEPOLIA_HOST = 0x1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95;
    address public BASE_SEPOLIA_HOST = 0x4c876500A13cc3825D343b5Ac791d3A4913bF14f;
    address public BSC_TESTNET_HOST = 0x022DDE07A21d8c553978b006D93CDe68ac83e677;
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