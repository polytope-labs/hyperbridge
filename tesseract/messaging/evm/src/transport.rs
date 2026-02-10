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

//! Pluggable JSON-RPC transport configuration for [`EvmClient`](crate::EvmClient).
//!
//! [`RpcTransport`] selects the transport variant at construction time via the
//! [`EvmConfig`](crate::EvmConfig).  The `Tron` variant signals that TRON-specific
//! field stripping should be applied to JSON-RPC requests (removing `type` and
//! `accessList` fields that TRON's JSON-RPC proxy cannot parse).

use serde::{Deserialize, Serialize};

/// Selects which JSON-RPC transport [`EvmClient`](crate::EvmClient) uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcTransport {
	/// Standard HTTP transport.  Works with any Ethereum-compatible
	/// JSON-RPC endpoint.
	#[default]
	Standard,
	/// TRON-compatible transport.  Strips the `type` and `accessList` fields
	/// from every JSON-RPC request parameter object before forwarding,
	/// because TRON's JSON-RPC proxy rejects them with `"JSON parse error"`.
	Tron,
}

/// Remove `type` and `accessList` from any object inside a top-level JSON
/// array.
///
/// For `eth_call`, `eth_estimateGas`, and similar methods, the parameters are
/// sent as a JSON array whose first element is the transaction call object.
/// This function walks each element and strips the keys that TRON cannot parse.
pub fn strip_tx_type_fields(mut value: serde_json::Value) -> serde_json::Value {
	if let Some(arr) = value.as_array_mut() {
		for item in arr.iter_mut() {
			if let Some(obj) = item.as_object_mut() {
				obj.remove("type");
				obj.remove("accessList");
			}
		}
	}
	value
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn strips_type_and_access_list() {
		let input = json!([
			{"to": "0xdead", "data": "0x1234", "type": "0x02", "accessList": []},
			"latest"
		]);
		let output = strip_tx_type_fields(input);
		let obj = output[0].as_object().unwrap();
		assert!(!obj.contains_key("type"));
		assert!(!obj.contains_key("accessList"));
		assert!(obj.contains_key("to"));
		assert!(obj.contains_key("data"));
		assert_eq!(output[1], "latest");
	}

	#[test]
	fn strips_legacy_type_field() {
		let input = json!([
			{"to": "0xdead", "data": "0x1234", "type": "0x00"},
			"latest"
		]);
		let output = strip_tx_type_fields(input);
		let obj = output[0].as_object().unwrap();
		assert!(!obj.contains_key("type"));
	}

	#[test]
	fn leaves_clean_params_alone() {
		let input = json!([
			{"to": "0xdead", "data": "0x1234"},
			"latest"
		]);
		let output = strip_tx_type_fields(input.clone());
		assert_eq!(input, output);
	}

	#[test]
	fn handles_non_array_params() {
		let input = json!({"method": "test"});
		let output = strip_tx_type_fields(input.clone());
		assert_eq!(input, output);
	}

	#[test]
	fn handles_null_params() {
		let input = json!(null);
		let output = strip_tx_type_fields(input.clone());
		assert_eq!(input, output);
	}

	#[test]
	fn rpc_transport_default_is_standard() {
		assert_eq!(RpcTransport::default(), RpcTransport::Standard);
	}

	#[test]
	fn rpc_transport_deserializes() {
		let standard: RpcTransport = serde_json::from_str("\"standard\"").unwrap();
		assert_eq!(standard, RpcTransport::Standard);

		let tron: RpcTransport = serde_json::from_str("\"tron\"").unwrap();
		assert_eq!(tron, RpcTransport::Tron);
	}
}
