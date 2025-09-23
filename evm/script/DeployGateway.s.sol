// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {ERC6160Ext20} from "@polytope-labs/erc6160/tokens/ERC6160Ext20.sol";
import {IERC6160Ext20} from "@polytope-labs/erc6160/interfaces/IERC6160Ext20.sol";
import {TokenGateway, Asset, TokenGatewayParamsExt, TokenGatewayParams, AssetMetadata} from "../src/modules/TokenGateway.sol";
import {TokenFaucet} from "../src/modules/TokenFaucet.sol";
import {CrossChainInscription} from "../src/modules/Inscriptions.sol";
import {CallDispatcher} from "../src/modules/CallDispatcher.sol";
import {BaseScript} from "./BaseScript.sol";
import {IIsmpHost} from "@polytope-labs/ismp-solidity/IIsmpHost.sol";

contract DeployScript is BaseScript {
    using strings for *;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    function run() external {
        vm.startBroadcast(uint256(privateKey));

        CallDispatcher callDispatcher = new CallDispatcher{salt: salt}();

        TokenGateway gateway = new TokenGateway{salt: salt}(admin);
        AssetMetadata[] memory assets = new AssetMetadata[](0);
        gateway.init(
            TokenGatewayParamsExt({
                params: TokenGatewayParams({host: HOST_ADDRESS, dispatcher: address(callDispatcher)}),
                assets: assets
            })
        );
    }
}
