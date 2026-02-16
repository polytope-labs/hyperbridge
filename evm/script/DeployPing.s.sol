// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import "stringutils/strings.sol";

import {PingModule} from "../src/utils/PingModule.sol";
import {BaseScript} from "./BaseScript.sol";

contract DeployScript is BaseScript {
    using strings for *;

    /// @notice Main deployment logic - called by BaseScript's run() functions
    /// @dev This function is called within a broadcast context
    function deploy() internal override {
        address tokenFaucet = config.get("TOKEN_FAUCET").toAddress();
        console.log("Token Faucet:", tokenFaucet);

        PingModule ping = new PingModule{salt: salt}(admin);
        console.log("PingModule deployed at:", address(ping));

        ping.setIsmpHost(HOST_ADDRESS, tokenFaucet);
        console.log("ISMP Host configured");
    }
}
