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

//! Pluggable JSON-RPC transport for [`EvmClient`](crate::EvmClient).
//!
//! When [`RpcTransport::Tron`] is selected, a [`TronLayer`] is applied to the
//! alloy RPC client.  This Tower layer intercepts every outgoing JSON-RPC
//! request and strips the `type` and `accessList` fields that TRON's JSON-RPC
//! proxy cannot parse.

use alloy_json_rpc::{Request, RequestPacket, ResponsePacket, SerializedRequest};
use alloy_transport::{TransportError, TransportErrorKind};
use serde::{Deserialize, Serialize};
use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};
use tower::{Layer, Service};

/// RPC methods whose params contain transaction-like objects with `type`/`accessList`.
const TX_OBJECT_METHODS: &[&str] = &[
	"eth_call",
	"eth_estimateGas",
	"eth_sendTransaction",
	"eth_signTransaction",
	"debug_traceCall",
];

/// Selects which JSON-RPC transport [`EvmClient`](crate::EvmClient) uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcTransport {
	/// Standard HTTP transport.  Works with any Ethereum-compatible
	/// JSON-RPC endpoint.
	#[default]
	Standard,
	/// TRON-compatible transport.  Applies [`TronLayer`] to strip EIP-1559
	/// fields (`type`, `accessList`) that TRON's JSON-RPC proxy rejects.
	Tron,
}

/// Tower layer that strips `type` and `accessList` fields from JSON-RPC
/// request parameters for TRON compatibility.
#[derive(Debug, Clone, Copy)]
pub struct TronLayer;

impl<S> Layer<S> for TronLayer {
	type Service = TronService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		TronService { inner }
	}
}

/// Tower service that strips incompatible fields from JSON-RPC requests.
#[derive(Debug, Clone)]
pub struct TronService<S> {
	inner: S,
}

impl<S> Service<RequestPacket> for TronService<S>
where
	S: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>
		+ Send
		+ Clone
		+ 'static,
	S::Future: Send + 'static,
{
	type Response = ResponsePacket;
	type Error = TransportError;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: RequestPacket) -> Self::Future {
		let modified = match req {
			RequestPacket::Single(ser_req) => strip_request(ser_req).map(RequestPacket::Single),
			RequestPacket::Batch(batch) => batch
				.into_iter()
				.map(strip_request)
				.collect::<Result<Vec<_>, _>>()
				.map(RequestPacket::Batch),
		};

		match modified {
			Ok(req) => {
				let fut = self.inner.call(req);
				Box::pin(fut)
			},
			Err(e) => Box::pin(async move { Err(e) }),
		}
	}
}

/// Strip `type` and `accessList` from the params of a single JSON-RPC request.
/// Only processes methods that send transaction-like objects; all others pass through
/// with zero overhead.
fn strip_request(req: SerializedRequest) -> Result<SerializedRequest, TransportError> {
	if !TX_OBJECT_METHODS.contains(&req.method()) {
		return Ok(req);
	}

	let params_raw = match req.params() {
		Some(raw) => raw.get().to_owned(),
		None => return Ok(req),
	};

	let mut params_value: serde_json::Value = serde_json::from_str(&params_raw)
		.map_err(|e| TransportErrorKind::custom(e))?;

	let modified = if let Some(arr) = params_value.as_array_mut() {
		let mut changed = false;
		for item in arr.iter_mut() {
			if let Some(obj) = item.as_object_mut() {
				changed |= obj.remove("type").is_some();
				changed |= obj.remove("accessList").is_some();
				// Tron uses `data` instead of `input`
				if let Some(input_val) = obj.remove("input") {
					obj.insert("data".to_string(), input_val);
					changed = true;
				}
			}
		}
		changed
	} else {
		false
	};

	if !modified {
		return Ok(req);
	}

	// Reconstruct the SerializedRequest with modified params
	let (meta, _) = req.decompose();
	let new_params = serde_json::value::to_raw_value(&params_value)
		.map_err(|e| TransportErrorKind::custom(e))?;
	let new_request = Request { meta, params: new_params };
	new_request.serialize().map_err(|e| TransportErrorKind::custom(e))
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	fn make_serialized_request(
		method: &'static str,
		params: serde_json::Value,
	) -> SerializedRequest {
		let raw_params = serde_json::value::to_raw_value(&params).unwrap();
		let request = alloy_json_rpc::Request {
			meta: alloy_json_rpc::RequestMeta::new(method.into(), alloy_json_rpc::Id::Number(1)),
			params: raw_params,
		};
		request.serialize().unwrap()
	}

	#[test]
	fn strips_type_and_access_list() {
		let req = make_serialized_request(
			"eth_call",
			json!([
				{"to": "0xdead", "data": "0x1234", "type": "0x02", "accessList": []},
				"latest"
			]),
		);
		let result = strip_request(req).unwrap();
		let params: serde_json::Value =
			serde_json::from_str(result.params().unwrap().get()).unwrap();
		let obj = params[0].as_object().unwrap();
		assert!(!obj.contains_key("type"));
		assert!(!obj.contains_key("accessList"));
		assert!(obj.contains_key("to"));
		assert!(obj.contains_key("data"));
		assert_eq!(params[1], "latest");
	}

	#[test]
	fn strips_legacy_type_field() {
		let req = make_serialized_request(
			"eth_estimateGas",
			json!([{"to": "0xdead", "data": "0x1234", "type": "0x00"}, "latest"]),
		);
		let result = strip_request(req).unwrap();
		let params: serde_json::Value =
			serde_json::from_str(result.params().unwrap().get()).unwrap();
		let obj = params[0].as_object().unwrap();
		assert!(!obj.contains_key("type"));
	}

	#[test]
	fn leaves_clean_params_alone() {
		let req = make_serialized_request(
			"eth_call",
			json!([{"to": "0xdead", "data": "0x1234"}, "latest"]),
		);
		let original_params = req.params().unwrap().get().to_owned();
		let result = strip_request(req).unwrap();
		let result_params = result.params().unwrap().get().to_owned();
		assert_eq!(original_params, result_params);
	}

	#[test]
	fn skips_non_tx_methods() {
		let req = make_serialized_request(
			"eth_getBlockByNumber",
			json!(["0x1", true]),
		);
		let original_params = req.params().unwrap().get().to_owned();
		let result = strip_request(req).unwrap();
		let result_params = result.params().unwrap().get().to_owned();
		assert_eq!(original_params, result_params);
	}

	#[test]
	fn renames_input_to_data() {
		let req = make_serialized_request(
			"eth_call",
			json!([{"to": "0xdead", "input": "0x1234"}, "latest"]),
		);
		let result = strip_request(req).unwrap();
		let params: serde_json::Value =
			serde_json::from_str(result.params().unwrap().get()).unwrap();
		let obj = params[0].as_object().unwrap();
		assert!(!obj.contains_key("input"));
		assert_eq!(obj.get("data").unwrap(), "0x1234");
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
