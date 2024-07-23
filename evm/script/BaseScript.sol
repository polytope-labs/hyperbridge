// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";

abstract contract BaseScript is Script {
	bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));

    address payable internal SEPOLIA_HOST = payable(vm.envAddress("SEPOLIA_HOST"));
    address payable internal ARB_SEPOLIA_HOST = payable(vm.envAddress("ARB_SEPOLIA_HOST"));
    address payable internal OP_SEPOLIA_HOST = payable(vm.envAddress("OP_SEPOLIA_HOST"));
    address payable internal BASE_SEPOLIA_HOST = payable(vm.envAddress("BASE_SEPOLIA_HOST"));
    address payable internal BSC_TESTNET_HOST = payable(vm.envAddress("BSC_TESTNET_HOST"));
    address payable internal FEE_TOKEN = payable(vm.envAddress("FEE_TOKEN"));



        /**
         * @dev Returns true if the two strings are equal.
         */
        function equal(string memory a, string memory b) internal pure returns (bool) {
            return bytes(a).length == bytes(b).length && keccak256(bytes(a)) == keccak256(bytes(b));
        }
}
