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

import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";

library StateMachine {
	/// @notice The identifier for the relay chain.
	uint256 public constant RELAY_CHAIN = 0;

	// @notice Address a state machine on the polkadot relay chain
	function polkadot(uint256 id) internal pure returns (bytes memory) {
		return bytes(string.concat("POLKADOT-", Strings.toString(id)));
	}

	// @notice Address a state machine on the kusama relay chain
	function kusama(uint256 id) internal pure returns (bytes memory) {
		return bytes(string.concat("KUSAMA-", Strings.toString(id)));
	}

	// @notice Address an evm state machine
	function evm(uint chainid) internal pure returns (bytes memory) {
		return bytes(string.concat("EVM-", Strings.toString(chainid)));
	}

	// @notice Address a substrate state machine
	function substrate(bytes4 id) internal pure returns (bytes memory) {
		return bytes(string.concat("SUBSTRATE-", string(abi.encodePacked(id))));
	}

	// @notice Address a tendermint state machine
	function tendermint(bytes4 id) internal pure returns (bytes memory) {
		return bytes(string.concat("TNDRMINT-", string(abi.encodePacked(id))));
	}
}
