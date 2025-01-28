// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {IERC6160Ext20} from "@polytope-labs/erc6160/interfaces/IERC6160Ext20.sol";
import {TokenGateway, Asset, TokenGatewayParamsExt, TokenGatewayParams, AssetMetadata} from "../src/modules/TokenGateway.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";
import {CrossChainInscription} from "../src/modules/Inscriptions.sol";
import {BaseScript} from "./BaseScript.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";

contract DeployScript is BaseScript {
    using strings for *;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        // todo:
        address callDispatcher = address(0);

        if (host.toSlice().startsWith("ethereum".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(ETHEREUM_HOST, admin);
            deployGateway(ETHEREUM_HOST, admin, callDispatcher);
        } else if (host.toSlice().startsWith("arbitrum".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(ARBITRUM_HOST, admin);
            deployGateway(ARBITRUM_HOST, admin, callDispatcher);
        } else if (host.toSlice().startsWith("optimism".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(OPTIMISM_HOST, admin);
            deployGateway(OPTIMISM_HOST, admin, callDispatcher);
        } else if (host.toSlice().startsWith("base".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(BASE_HOST, admin);
            deployGateway(BASE_HOST, admin, callDispatcher);
        } else if (host.toSlice().startsWith("bsc".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(BNB_HOST, admin);
            deployGateway(BNB_HOST, admin, callDispatcher);
        } else if (host.toSlice().startsWith("gnosis".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(GNOSIS_HOST, admin);
            deployGateway(GNOSIS_HOST, admin, callDispatcher);
        } else if (host.toSlice().startsWith("soneium".toSlice())) {
            vm.startBroadcast(uint256(privateKey));
            deployInscription(SONEIUM_HOST, admin);
            deployGateway(SONEIUM_HOST, admin, callDispatcher);
        }
    }

    function deployInscription(address host, address admin) public {
        CrossChainInscription c = new CrossChainInscription{salt: salt}(admin);
        c.setHost(host);
    }

    function deployGateway(address host, address admin, address callDispatcher) public {
        TokenGateway gateway = new TokenGateway{salt: salt}(admin);

        AssetMetadata[] memory assets = new AssetMetadata[](0);

        gateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({host: host, dispatcher: callDispatcher}),
                assets: assets
            })
        );
    }
}
