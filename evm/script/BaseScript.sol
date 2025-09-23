// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";

abstract contract BaseScript is Script {
    address payable internal ETHEREUM_HOST = payable(vm.envAddress("ETHEREUM_HOST"));
    address payable internal ARBITRUM_HOST = payable(vm.envAddress("ARBITRUM_HOST"));
    address payable internal OPTIMISM_HOST = payable(vm.envAddress("OPTIMISM_HOST"));
    address payable internal BASE_HOST = payable(vm.envAddress("BASE_HOST"));
    address payable internal BNB_HOST = payable(vm.envAddress("BNB_HOST"));
    address payable internal GNOSIS_HOST = payable(vm.envAddress("GNOSIS_HOST"));
    address payable internal SONEIUM_HOST = payable(vm.envAddress("SONEIUM_HOST"));
    address payable internal POLYGON_HOST = payable(vm.envAddress("POLYGON_HOST"));
    address payable internal UNICHAIN_HOST = payable(vm.envAddress("UNICHAIN_HOST"));

    bytes32 internal privateKey = vm.envBytes32("PRIVATE_KEY");
    address internal admin = vm.envAddress("ADMIN");
    bytes internal consensusState = vm.envBytes("CONSENSUS_STATE");
    string internal host = vm.envString("HOST");
    address internal HOST_ADDRESS = vm.envAddress(string.concat(host, "_HOST"));
    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));

    /**
     * @dev Returns true if the two strings are equal.
     */
    function equal(string memory a, string memory b) internal pure returns (bool) {
        return bytes(a).length == bytes(b).length && keccak256(bytes(a)) == keccak256(bytes(b));
    }
}
