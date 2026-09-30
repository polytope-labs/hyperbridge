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
pragma solidity ^0.8.24;

import {IntentGatewayV2} from "../../src/apps/IntentGatewayV2.sol";
import {IntrinsicModule} from "../../src/apps/intentsv2/IntrinsicModule.sol";
import {ExtrinsicModule} from "../../src/apps/intentsv2/ExtrinsicModule.sol";

/**
 * @dev Fresh modules for a test implementation.
 */
function deployIntentModules() returns (address intrinsic, address extrinsic) {
    intrinsic = address(new IntrinsicModule());
    extrinsic = address(new ExtrinsicModule());
}

/**
 * @dev A raw `IntentGatewayV2` implementation with fresh modules, ready to sit behind a proxy.
 */
function deployIntentGatewayImpl() returns (IntentGatewayV2) {
    (address intrinsic, address extrinsic) = deployIntentModules();
    return new IntentGatewayV2(intrinsic, extrinsic);
}
