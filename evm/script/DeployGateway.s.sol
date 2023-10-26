// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "multi-chain-tokens/tokens/ERC20.sol";

import "../src/modules/TokenGateway.sol";
import "../src/modules/TokenFaucet.sol";
import "../test/PingModule.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes("gargantua-v0.0.4"));

    address public GOERLI_HOST = 	0xDaC0797eb874d7a4A53521DD16250fbEb85797f0;
    address public ARB_GOERLI_HOST = 0xa8070743D9e2B4aa3dEF52ed04A8e045F16C3252;
    address public OP_GOERLI_HOST = 0xB8D705737d63Ce49ec8c491b968D29F497D431f1;
    address public BASE_GOERLI_HOST = 0x5Cd82e710385e7e14c5fa97B9Ceae31150Be8dFd;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");

        vm.createSelectFork("goerli");
        vm.startBroadcast(uint256(privateKey));
        deployGateway(GOERLI_HOST, admin);
        vm.stopBroadcast();

        vm.createSelectFork("arbitrum-goerli");
        vm.startBroadcast(uint256(privateKey));
        deployGateway(ARB_GOERLI_HOST, admin);
        vm.stopBroadcast();

        vm.createSelectFork("optimism-goerli");
        vm.startBroadcast(uint256(privateKey));
        deployGateway(OP_GOERLI_HOST, admin);
        vm.stopBroadcast();

        vm.createSelectFork("base-goerli");
        vm.startBroadcast(uint256(privateKey));
        deployGateway(BASE_GOERLI_HOST, admin);
        vm.stopBroadcast();
    }

    function deployGateway(address host, address admin) public {
        MultiChainNativeERC20 t = new MultiChainNativeERC20{ salt: salt }(admin, "Hyperbridge Test Token", "CORE");

        TokenGateway gateway = new TokenGateway{ salt: salt }(admin);
        gateway.setIsmpHost(host);
        t.grantRole(MINTER_ROLE, address(gateway));
        t.grantRole(BURNER_ROLE, address(gateway));

        TokenFaucet faucet = new TokenFaucet{salt: salt}(address(t));
        t.grantRole(MINTER_ROLE, address(faucet));
    }
}
