// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "multi-chain-tokens/tokens/ERC20.sol";

import "../src/modules/TokenGateway.sol";
import "../src/modules/TokenFaucet.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes("gargantuan_v0"));

    address public GOERLI_HOST = 0x4a23BF364332dC8d8Dd81552466c7d267D20e988;
    address public ARB_GOERLI_HOST = 0xb58F8D53c8e55345d3A620094670B0C3892a097b;
    address public OP_GOERLI_HOST = 0x4e74812F70A40328F3703740F03cE8d6208f0CEC;
    address public BASE_GOERLI_HOST = 0x9ff290c1650423EF5BA96eE066604Ca4c457C79C;

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
