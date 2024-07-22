// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import "openzeppelin/utils/Strings.sol";
import {ERC6160Ext20} from "ERC6160/tokens/ERC6160Ext20.sol";
import {IERC6160Ext20} from "ERC6160/interfaces/IERC6160Ext20.sol";
import {
    TokenGateway,
    Asset,
    TokenGatewayParamsExt,
    TokenGatewayParams,
    AssetMetadata
} from "../contracts/modules/TokenGateway.sol";
import {TokenFaucet} from "../contracts/modules/TokenFaucet.sol";
import {CrossChainMessenger} from "../examples/CrossChainMessenger.sol";

contract DeployScript is Script {
    bytes32 public salt = keccak256(bytes(vm.envString("VERSION")));

    address public SEPOLIA_HOST = vm.envAddress("SEPOLIA_HOST");
    address public ARB_SEPOLIA_HOST = vm.envAddress("ARB_SEPOLIA_HOST");
    address public OP_SEPOLIA_HOST = vm.envAddress("OP_SEPOLIA_HOST");
    address public BASE_SEPOLIA_HOST = vm.envAddress("BASE_SEPOLIA_HOST");
    address public BSC_TESTNET_HOST = vm.envAddress("BSC_TESTNET_HOST");
    address public FEE_TOKEN = vm.envAddress("FEE_TOKEN");

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        string memory host = vm.envString("HOST");
        // todo:
        address uniRouter = address(1);
        address callDispatcher = address(1);

        if (Strings.equal(host, "sepolia") || Strings.equal(host, "ethereum")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "arbitrum-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(ARB_SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "optimism-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(OP_SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "base-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BASE_SEPOLIA_HOST, admin, uniRouter, callDispatcher);
        } else if (Strings.equal(host, "bsc-testnet")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BSC_TESTNET_HOST, admin, uniRouter, callDispatcher);
        }
    }

    function deployMessenger(address host, address admin) public {
        CrossChainMessenger c = new CrossChainMessenger{salt: salt}(admin);
        c.setIsmpHost(host);
    }

    function deployGateway(address host, address admin, address uniRouter, address callDispatcher) public {
        uint256 _paraId = vm.envUint("PARA_ID");

        IERC6160Ext20 feeToken = IERC6160Ext20(FEE_TOKEN);

        TokenGateway gateway = new TokenGateway{salt: salt}(admin);
        feeToken.grantRole(MINTER_ROLE, address(gateway));
        feeToken.grantRole(BURNER_ROLE, address(gateway));

        AssetMetadata[] memory assets = new AssetMetadata[](1);
        assets[0] = AssetMetadata({
            erc20: address(0),
            erc6160: address(feeToken),
            name: "Hyperbridge USD",
            symbol: "USDH",
            beneficiary: address(0),
            initialSupply: 0
        });

        gateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({host: host, uniswapV2: uniRouter, dispatcher: callDispatcher}),
                assets: assets
            })
        );
    }
}
