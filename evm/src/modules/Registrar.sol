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
pragma solidity 0.8.17;

import {BaseIsmpModule, PostRequest, IncomingPostRequest} from "ismp/IIsmpModule.sol";

contract TokenGatewayRegistrar is BaseIsmpModule {
    // Serves as gas abstraction for registering assets on the Hyperbridge chain
    // by collecting fees here and depositing to the host.
    function beginAssetRegistration(bytes32 assetId) public payable {
        // either the user supplies the native asset or they should have approved
        // the required amount in feeToken() here

        // dispatches a request to hyperbridge that allows hyperbridge permit unsigned transactions.
    }
}
