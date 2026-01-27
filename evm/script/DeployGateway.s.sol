// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {HyperFungibleTokenImpl} from "../src/utils/HyperFungibleTokenImpl.sol";
import {TokenGateway, TokenGatewayParams, AssetMetadata} from "../src/apps/TokenGateway.sol";
import {TokenFaucet} from "../src/utils/TokenFaucet.sol";
import {CrossChainInscription} from "../src/utils/Inscriptions.sol";
import {CallDispatcher} from "../src/utils/CallDispatcher.sol";
import {BaseScript} from "./BaseScript.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";

contract DeployScript is BaseScript {
    using strings for *;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER ROLE");

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        CallDispatcher callDispatcher = new CallDispatcher{salt: salt}();
        console.log("CallDispatcher deployed at:", address(callDispatcher));

        TokenGateway gateway = new TokenGateway{salt: salt}(admin);
        console.log("TokenGateway deployed at:", address(gateway));

        gateway.init(TokenGatewayParams({host: HOST_ADDRESS, dispatcher: address(callDispatcher)}));
        console.log("TokenGateway initialized");
    }
}
