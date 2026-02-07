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

//! Pluggable JSON-RPC transport layer for [`EvmClient`](crate::EvmClient).
//!
//! [`OmniClient`] is an enum that dispatches JSON-RPC calls to either the
//! standard ethers [`Http`] transport or [`TronJsonRpc`], a thin wrapper
//! that strips EIP-1559 fields (`type`, `accessList`) which TRON's JSON-RPC
//! proxy cannot parse.
//!
//! The variant is selected at construction time via [`RpcTransport`] in the
//! [`EvmConfig`](crate::EvmConfig).

use async_trait::async_trait;
use ethers::providers::{Http, JsonRpcClient, ProviderError, RpcError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Type alias for the error type produced by the [`Http`] JSON-RPC client.
type HttpError = <Http as JsonRpcClient>::Error;

/// Selects which JSON-RPC transport [`EvmClient`](crate::EvmClient) uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcTransport {
	/// Standard ethers [`Http`] transport.  Works with any Ethereum-compatible
	/// JSON-RPC endpoint.
	#[default]
	Standard,
	/// TRON-compatible transport.  Wraps [`Http`] and strips the `type` and
	/// `accessList` fields from every JSON-RPC request parameter object before
	/// forwarding, because TRON's JSON-RPC proxy rejects them with
	/// `"JSON parse error"`.
	Tron,
}

/// A JSON-RPC client that wraps [`Http`] and removes EVM transaction-type
/// fields (`type`, `accessList`) that TRON's JSON-RPC proxy cannot parse.
///
/// Every outbound request is intercepted: its parameters are serialised to
/// [`serde_json::Value`], any top-level object in a JSON array has the
/// offending keys removed, and the sanitised value is forwarded to the
/// inner [`Http`] transport.
pub struct TronJsonRpc {
	inner: Http,
}

impl TronJsonRpc {
	/// Wrap an existing [`Http`] transport.
	pub fn new(inner: Http) -> Self {
		Self { inner }
	}
}

impl fmt::Debug for TronJsonRpc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("TronJsonRpc").field("inner", &self.inner).finish()
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for TronJsonRpc {
	type Error = HttpError;

	async fn request<T, R>(&self, method: &str, params: T) -> Result<R, Self::Error>
	where
		T: fmt::Debug + Serialize + Send + Sync,
		R: DeserializeOwned + Send,
	{
		// Serialise → strip → forward.  The extra serialisation round-trip is
		// negligible compared to the network RTT.
		let value = serde_json::to_value(&params)
			.map_err(|e| ProviderError::SerdeJson(e))
			.map_err(|e| match e {
				ProviderError::SerdeJson(e) => HttpError::SerdeJson {
					err: e,
					text: "failed to serialize params for field stripping".into(),
				},
				_ => unreachable!(),
			})?;
		let cleaned = strip_tx_type_fields(value);
		self.inner.request(method, cleaned).await
	}
}

/// A unified JSON-RPC client that dispatches to either [`Http`] (standard) or
/// [`TronJsonRpc`] (TRON-compatible).
///
/// `EvmClient` stores `Provider<OmniClient>` so that the transport can be
/// selected at runtime via [`RpcTransport`] in the config, without changing
/// any downstream type signatures.
pub enum OmniClient {
	/// Standard ethers HTTP transport.
	Http(Http),
	/// TRON-compatible transport that strips EIP-1559 fields.
	Tron(TronJsonRpc),
}

impl Clone for TronJsonRpc {
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone() }
	}
}

impl Clone for OmniClient {
	fn clone(&self) -> Self {
		match self {
			OmniClient::Http(inner) => OmniClient::Http(inner.clone()),
			OmniClient::Tron(inner) => OmniClient::Tron(inner.clone()),
		}
	}
}

impl OmniClient {
	/// Construct an [`OmniClient`] from an [`Http`] transport and a
	/// [`RpcTransport`] selector.
	pub fn new(http: Http, transport: &RpcTransport) -> Self {
		match transport {
			RpcTransport::Standard => OmniClient::Http(http),
			RpcTransport::Tron => OmniClient::Tron(TronJsonRpc::new(http)),
		}
	}
}

impl fmt::Debug for OmniClient {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			OmniClient::Http(inner) => f.debug_tuple("OmniClient::Http").field(inner).finish(),
			OmniClient::Tron(inner) => f.debug_tuple("OmniClient::Tron").field(inner).finish(),
		}
	}
}

/// Unified error type for [`OmniClient`].
///
/// Both variants use the same underlying [`Http`] error since [`TronJsonRpc`]
/// delegates to [`Http`] after stripping fields.
#[derive(Debug, Error)]
pub enum OmniClientError {
	#[error(transparent)]
	Http(HttpError),
}

impl RpcError for OmniClientError {
	fn as_error_response(&self) -> Option<&ethers::providers::JsonRpcError> {
		match self {
			OmniClientError::Http(e) => e.as_error_response(),
		}
	}

	fn as_serde_error(&self) -> Option<&serde_json::Error> {
		match self {
			OmniClientError::Http(e) => e.as_serde_error(),
		}
	}
}

impl From<OmniClientError> for ProviderError {
	fn from(err: OmniClientError) -> Self {
		match err {
			OmniClientError::Http(e) => e.into(),
		}
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for OmniClient {
	type Error = OmniClientError;

	async fn request<T, R>(&self, method: &str, params: T) -> Result<R, Self::Error>
	where
		T: fmt::Debug + Serialize + Send + Sync,
		R: DeserializeOwned + Send,
	{
		match self {
			OmniClient::Http(inner) =>
				inner.request(method, params).await.map_err(OmniClientError::Http),
			OmniClient::Tron(inner) =>
				inner.request(method, params).await.map_err(OmniClientError::Http),
		}
	}
}

/// Remove `type` and `accessList` from any object inside a top-level JSON
/// array.
///
/// For `eth_call`, `eth_estimateGas`, and similar methods, ethers sends the
/// parameters as a JSON array whose first element is the transaction call
/// object.  This function walks each element and strips the keys that TRON
/// cannot parse.
fn strip_tx_type_fields(mut value: serde_json::Value) -> serde_json::Value {
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
