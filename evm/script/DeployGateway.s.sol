// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "multi-chain-tokens/tokens/ERC20.sol";

import "../src/modules/TokenGateway.sol";
import "../src/modules/TokenFaucet.sol";
import "../examples/PingModule.sol";
import "../examples/CrossChainMessenger.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes("gargantua-v0.0.7"));

    address public SEPOLIA_HOST = 0x5b5F63C8f3985CaFE1CE53E6374f42AB60dE5a6B;
    address public ARB_SEPOLIA_HOST = 0x43E136611Cf74E165116a47e6F9C58AFCc80Ec54;
    address public OP_SEPOLIA_HOST = 0x0124f458900FCd101c4CE31A9772fD2c5e6d65BF;
    address public BASE_SEPOLIA_HOST = 0x87825f839d95c6021c0e821917F93aDB299eD6F8;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        string memory host = vm.envString("HOST");

        if (Strings.equal(host, "sepolia") || Strings.equal(host, "ethereum")) {
            vm.createSelectFork("sepolia");
            vm.startBroadcast(uint256(privateKey));
            deployGateway(SEPOLIA_HOST, admin);
            vm.stopBroadcast();
        } else if (Strings.equal(host, "arbitrum-sepolia")) {
            vm.createSelectFork("arbitrum-sepolia");
            vm.startBroadcast(uint256(privateKey));
            deployGateway(ARB_SEPOLIA_HOST, admin);
            vm.stopBroadcast();
        } else if (Strings.equal(host, "optimism-sepolia")) {
            vm.createSelectFork("optimism-sepolia");
            vm.startBroadcast(uint256(privateKey));
            deployGateway(OP_SEPOLIA_HOST, admin);
            vm.stopBroadcast();
        } else if (Strings.equal(host, "base-sepolia")) {
            vm.createSelectFork("base-sepolia");
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BASE_SEPOLIA_HOST, admin);
            vm.stopBroadcast();
        }
    }

    function deployMessenger(address host, address admin) public {
        CrossChainMessenger c = new CrossChainMessenger{salt: salt}(admin);
        c.setIsmpHost(host);
    }

    function deployGateway(address host, address admin) public {
        MultiChainNativeERC20 t = new MultiChainNativeERC20{salt: salt}(admin, "Hyperbridge Test Token", "CORE");

        TokenGateway gateway = new TokenGateway{salt: salt}(admin);
        gateway.setIsmpHost(host);
        t.grantRole(MINTER_ROLE, address(gateway));
        t.grantRole(BURNER_ROLE, address(gateway));

        TokenFaucet faucet = new TokenFaucet{salt: salt}(address(t));
        t.grantRole(MINTER_ROLE, address(faucet));
    }
}
