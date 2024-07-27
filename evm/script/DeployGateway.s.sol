// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {IERC6160Ext20} from "@polytope-labs/erc6160/interfaces/IERC6160Ext20.sol";
import {TokenGateway, Asset, TokenGatewayParamsExt, TokenGatewayParams, AssetMetadata} from "../src/modules/TokenGateway.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";
import {CrossChainMessenger} from "../examples/CrossChainMessenger.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        address admin = vm.envAddress("ADMIN");
        bytes32 privateKey = vm.envBytes32("PRIVATE_KEY");
        string memory host = vm.envString("HOST");
        // todo:
        address uniRouter = address(1);
        address callDispatcher = address(1);

        if (equal(host, "sepolia") || equal(host, "ethereum")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(SEPOLIA_HOST, admin, callDispatcher);
        } else if (equal(host, "arbitrum-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(ARB_SEPOLIA_HOST, admin, callDispatcher);
        } else if (equal(host, "optimism-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(OP_SEPOLIA_HOST, admin, callDispatcher);
        } else if (equal(host, "base-sepolia")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BASE_SEPOLIA_HOST, admin, callDispatcher);
        } else if (equal(host, "bsc-testnet")) {
            vm.startBroadcast(uint256(privateKey));
            deployGateway(BSC_TESTNET_HOST, admin, callDispatcher);
        }
    }

    function deployMessenger(address host, address admin) public {
        CrossChainMessenger c = new CrossChainMessenger{salt: salt}(admin);
        c.setIsmpHost(host);
    }

    function deployGateway(address host, address admin, address callDispatcher) public {
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
                params: TokenGatewayParams({host: host, dispatcher: callDispatcher}),
                assets: assets
            })
        );
    }
}
