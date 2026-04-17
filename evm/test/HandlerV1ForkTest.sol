// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {IHandler} from "@hyperbridge/core/interfaces/IHandler.sol";
import {IHost} from "@hyperbridge/core/interfaces/IHost.sol";

contract HandlerV1ForkTest is Test {
    IHandler internal handler;
    IHost internal host;

    // TODO: Add deployed addresses
    address constant HANDLER_ADDRESS = 0xDa9aa832cF1024862a23f9fDd47cC2358B7d549c;
    address constant HOST_ADDRESS = 0x8Af30d750a0Be06fA60A3Cc61e1EE3Ad5766fE86;

    function setUp() public {
        // TODO: Set your Tron RPC URL env variable
        vm.createSelectFork("https://nile.trongrid.io/jsonrpc");

        handler = IHandler(HANDLER_ADDRESS);
        host = IHost(HOST_ADDRESS);
    }

    // function test_handlePostRequests() public {
    //     // TODO: Add your calldata
    //     bytes memory callData = hex"";

    //     (bool success, bytes memory returnData) = address(handler).call(callData);
    //     if (!success) {
    //         if (returnData.length > 0) {
    //             assembly {
    //                 revert(add(returnData, 32), mload(returnData))
    //             }
    //         }
    //         revert("Call failed");
    //     }
    // }


    function test_helloWorld() public {
        console.log("Hello World");
    }
}
