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

use alloy::transports::layers::FallbackService;
use alloy_json_rpc::{
	Request, RequestPacket, Response, ResponsePacket, ResponsePayload, SerializedRequest,
};
use alloy_primitives::Bytes;
use alloy_transport::{TransportError, TransportErrorKind, TransportFut};
use serde::{Deserialize, Serialize};
use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
	time::Duration,
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

/// Tower service that turns a JSON-RPC error response into a transport error.
///
/// A JSON-RPC error arrives as an HTTP 200, so the inner transport reports it as a success.
/// [`FallbackService`] returns the first success it sees, so without this wrapper a fast error
/// would beat a slower valid answer. A single-response error becomes
/// [`TransportError::ErrorResp`], the error callers already get from an error body; any other
/// packet, batches included, passes through unchanged.
#[derive(Debug, Clone)]
pub struct ErrorResponseAsFailure<S>(S);

impl<S> Service<RequestPacket> for ErrorResponseAsFailure<S>
where
	S: Service<
			RequestPacket,
			Response = ResponsePacket,
			Error = TransportError,
			Future = TransportFut<'static>,
		> + Send
		+ 'static,
{
	type Response = ResponsePacket;
	type Error = TransportError;
	type Future = TransportFut<'static>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.0.poll_ready(cx)
	}

	fn call(&mut self, req: RequestPacket) -> Self::Future {
		let fut = self.0.call(req);
		Box::pin(async move {
			match fut.await? {
				ResponsePacket::Single(Response {
					payload: ResponsePayload::Failure(err), ..
				}) => Err(TransportError::ErrorResp(err)),
				packet => Ok(packet),
			}
		})
	}
}

/// Backoff before each retry; at most one retry per entry.
const RETRY_BACKOFF: [Duration; 4] = [
	Duration::from_millis(250),
	Duration::from_millis(500),
	Duration::from_secs(1),
	Duration::from_secs(2),
];

/// Overall time budget for retries, measured from the start of the call.
const RETRY_BUDGET: Duration = Duration::from_secs(60);

/// Tower layer that retries transient JSON-RPC failures.
#[derive(Debug, Clone, Copy)]
pub struct RetryLayer;

impl<S> Layer<S> for RetryLayer {
	type Service = RetryService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		RetryService { inner }
	}
}

/// Tower service that retries transient failures with a bounded backoff.
///
/// Deterministic responses, such as reverts and invalid params, and transaction sends pass
/// through untouched. A retry never makes a call outlast [`RETRY_BUDGET`] plus its first
/// attempt, and the last completed result is returned when the budget runs out.
#[derive(Debug, Clone)]
pub struct RetryService<S> {
	inner: S,
}

impl<S> Service<RequestPacket> for RetryService<S>
where
	S: Service<
			RequestPacket,
			Response = ResponsePacket,
			Error = TransportError,
			Future = TransportFut<'static>,
		> + Clone
		+ Send
		+ 'static,
{
	type Response = ResponsePacket;
	type Error = TransportError;
	type Future = TransportFut<'static>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: RequestPacket) -> Self::Future {
		let inner = self.inner.clone();
		let mut inner = std::mem::replace(&mut self.inner, inner);

		// Resending a transaction after an ambiguous failure could duplicate it.
		let can_retry = !req.method_names().any(|method| {
			matches!(method, "eth_sendRawTransaction" | "eth_sendTransaction")
		});
		let is_batch = matches!(req, RequestPacket::Batch(_));
		let method = req
			.requests()
			.first()
			.map(|req| req.method().to_string())
			.unwrap_or_default();
		let id = req.requests().first().map(|req| req.id().clone());
		let raw_send = single_send_raw_transaction(&req);

		Box::pin(async move {
			let start = tokio::time::Instant::now();
			let mut result = inner.call(req.clone()).await;
			let mut retries = 0;

			while can_retry && result_is_transient(&result, &method, is_batch) {
				let Some(&backoff) = RETRY_BACKOFF.get(retries) else { break };
				let elapsed = start.elapsed();
				if elapsed + backoff >= RETRY_BUDGET {
					break
				}

				tracing::debug!(
					target: crate::LOG_TARGET,
					method = %method,
					attempt = retries + 1,
					backoff_ms = backoff.as_millis(),
					?result,
					"transient RPC failure, retrying",
				);
				tokio::time::sleep(backoff).await;
				retries += 1;

				let remaining = RETRY_BUDGET.saturating_sub(start.elapsed());
				match tokio::time::timeout(remaining, inner.call(req.clone())).await {
					Ok(response) => result = response,
					// The budget ran out; report the last attempt that completed.
					Err(_) => break,
				}
			}

			if let (Some(raw), Some(id)) = (raw_send, id) {
				if is_already_known(&result) {
					let hash = alloy_primitives::keccak256(raw);
					if let Ok(value) = serde_json::value::to_raw_value(&format!("{hash}")) {
						return Ok(ResponsePacket::Single(Response {
							id,
							payload: ResponsePayload::Success(value),
						}))
					}
				}
			}

			result
		})
	}
}

/// Whether a batch result, or a single response, should be attempted again.
fn result_is_transient(
	result: &Result<ResponsePacket, TransportError>,
	method: &str,
	is_batch: bool,
) -> bool {
	if is_batch {
		return result.is_err()
	}
	is_transient(result, method)
}

/// Whether `result` is a transient failure that an immediate retry may resolve.
fn is_transient(result: &Result<ResponsePacket, TransportError>, method: &str) -> bool {
	match result {
		Ok(ResponsePacket::Single(Response {
			payload: ResponsePayload::Failure(err), ..
		})) => transient_payload(err),
		Ok(ResponsePacket::Single(Response {
			payload: ResponsePayload::Success(raw), ..
		})) => method == "eth_getProof" && raw.get().trim() == "null",
		Err(TransportError::ErrorResp(err)) => transient_payload(err),
		Err(TransportError::Transport(kind)) => match kind {
			TransportErrorKind::HttpError(err) => {
				err.status == 429 || (500..=599).contains(&err.status)
			},
			TransportErrorKind::Custom(err) => {
				match err.downcast_ref::<alloy::transports::http::reqwest::Error>() {
					Some(err) => err.is_timeout() || err.is_connect(),
					None => kind.is_retry_err(),
				}
			},
			kind => kind.is_retry_err(),
		},
		_ => false,
	}
}

/// Whether a JSON-RPC error payload is transient.
fn transient_payload(payload: &alloy_json_rpc::ErrorPayload) -> bool {
	payload.is_retry_err() || {
		let message = payload.message.to_lowercase();
		["historical state", "missing trie node", "header not found"]
			.iter()
			.any(|needle| message.contains(needle))
	}
}

/// The raw transaction bytes of a single `eth_sendRawTransaction` request.
fn single_send_raw_transaction(req: &RequestPacket) -> Option<Bytes> {
	let RequestPacket::Single(req) = req else { return None };
	if req.method() != "eth_sendRawTransaction" {
		return None
	}
	let params: Vec<Bytes> = serde_json::from_str(req.params()?.get()).ok()?;
	params.into_iter().next()
}

/// Whether the error says the transaction is already known to the node.
fn is_already_known(result: &Result<ResponsePacket, TransportError>) -> bool {
	let message = match result {
		Ok(ResponsePacket::Single(Response {
			payload: ResponsePayload::Failure(err), ..
		})) => &err.message,
		Err(TransportError::ErrorResp(err)) => &err.message,
		_ => return false,
	};
	message.to_lowercase().contains("already known")
}

/// Builds a [`FallbackService`] over `transports` with every endpoint active.
///
/// Every endpoint is asked and the first successful answer wins. A JSON-RPC error from one
/// endpoint does not end the race, so a pruned node's missing-state error cannot beat an archive
/// node's proof; when every endpoint fails, the last error is returned. A request that one
/// endpoint answers with a JSON-RPC error now waits for the other endpoints, so a reverting
/// `eth_call` or `eth_estimateGas` returns at the slowest endpoint's latency. If another endpoint
/// hangs, the request waits for it, up to the HTTP client's timeout, and may then return that
/// endpoint's transport error rather than the JSON-RPC error.
pub fn fallback_service<S: Clone>(
	transports: Vec<S>,
) -> FallbackService<ErrorResponseAsFailure<S>> {
	let count = transports.len();
	FallbackService::new(transports.into_iter().map(ErrorResponseAsFailure).collect(), count)
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

	let mut params_value: serde_json::Value =
		serde_json::from_str(&params_raw).map_err(|e| TransportErrorKind::custom(e))?;

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
	use std::time::Duration;

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
		let req = make_serialized_request("eth_getBlockByNumber", json!(["0x1", true]));
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

	/// Answers every request with `payload` after `delay`.
	#[derive(Clone)]
	struct Endpoint {
		delay: Duration,
		payload: ResponsePayload,
	}

	impl Endpoint {
		fn ok(delay: Duration) -> Self {
			let value = serde_json::value::RawValue::from_string("\"0x1\"".into()).unwrap();
			Self { delay, payload: ResponsePayload::Success(value) }
		}

		fn error(delay: Duration) -> Self {
			let message = "historical state 0x1 is not available";
			let err =
				alloy_json_rpc::ErrorPayload { code: -32000, message: message.into(), data: None };
			Self { delay, payload: ResponsePayload::Failure(err) }
		}
	}

	impl Service<RequestPacket> for Endpoint {
		type Response = ResponsePacket;
		type Error = TransportError;
		type Future = TransportFut<'static>;

		fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}

		fn call(&mut self, _: RequestPacket) -> Self::Future {
			let Self { delay, payload } = self.clone();
			Box::pin(async move {
				tokio::time::sleep(delay).await;
				Ok(ResponsePacket::Single(Response { id: alloy_json_rpc::Id::Number(1), payload }))
			})
		}
	}

	async fn get_proof(endpoints: Vec<Endpoint>) -> Result<ResponsePacket, TransportError> {
		let request = make_serialized_request("eth_getProof", json!([]));
		fallback_service(endpoints).call(RequestPacket::Single(request)).await
	}

	#[tokio::test]
	async fn a_fast_error_response_does_not_beat_a_slower_answer() {
		let response = get_proof(vec![
			Endpoint::error(Duration::ZERO),
			Endpoint::ok(Duration::from_millis(50)),
		])
		.await
		.unwrap();

		let ResponsePacket::Single(Response { payload: ResponsePayload::Success(value), .. }) =
			response
		else {
			panic!("expected a single successful response, got {response:?}");
		};
		assert_eq!(value.get(), "\"0x1\"");
	}

	#[tokio::test]
	async fn every_endpoint_erroring_returns_the_error() {
		let err = get_proof(vec![
			Endpoint::error(Duration::ZERO),
			Endpoint::error(Duration::from_millis(10)),
		])
		.await
		.unwrap_err();

		let TransportError::ErrorResp(payload) = err else {
			panic!("expected a JSON-RPC error response, got {err:?}");
		};
		assert_eq!(payload.code, -32000);
	}

	/// Returns a queued result per call and counts how many times it was called.
	#[derive(Clone)]
	struct Scripted {
		script: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<(
			Duration,
			Result<ResponsePacket, TransportError>,
		)>>>,
		calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
	}

	impl Scripted {
		fn new(results: Vec<Result<ResponsePacket, TransportError>>) -> Self {
			Self::with_delays(results.into_iter().map(|result| (Duration::ZERO, result)).collect())
		}

		fn with_delays(script: Vec<(Duration, Result<ResponsePacket, TransportError>)>) -> Self {
			Self {
				script: std::sync::Arc::new(std::sync::Mutex::new(script.into())),
				calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
			}
		}
	}

	impl Service<RequestPacket> for Scripted {
		type Response = ResponsePacket;
		type Error = TransportError;
		type Future = TransportFut<'static>;

		fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}

		fn call(&mut self, _: RequestPacket) -> Self::Future {
			let this = self.clone();
			Box::pin(async move {
				this.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				let (delay, result) = this
					.script
					.lock()
					.unwrap()
					.pop_front()
					.expect("scripted endpoint exhausted");
				tokio::time::sleep(delay).await;
				result
			})
		}
	}

	fn success(value: &str) -> Result<ResponsePacket, TransportError> {
		let value = serde_json::value::RawValue::from_string(value.to_string()).unwrap();
		Ok(ResponsePacket::Single(Response {
			id: alloy_json_rpc::Id::Number(1),
			payload: ResponsePayload::Success(value),
		}))
	}

	fn failure(code: i64, message: &str) -> Result<ResponsePacket, TransportError> {
		let err =
			alloy_json_rpc::ErrorPayload { code, message: message.to_owned().into(), data: None };
		Ok(ResponsePacket::Single(Response {
			id: alloy_json_rpc::Id::Number(1),
			payload: ResponsePayload::Failure(err),
		}))
	}

	fn request(method: &'static str, params: serde_json::Value) -> RequestPacket {
		RequestPacket::Single(make_serialized_request(method, params))
	}

	fn retrying(endpoint: Scripted) -> RetryService<Scripted> {
		RetryLayer.layer(endpoint)
	}

	#[tokio::test(start_paused = true)]
	async fn retries_historical_state_errors() {
		let endpoint = Scripted::new(vec![
			failure(-32000, "historical state 0x1 is not available"),
			failure(-32000, "historical state 0x1 is not available"),
			success("\"0x1\""),
		]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
		assert!(matches!(
			response,
			ResponsePacket::Single(Response { payload: ResponsePayload::Success(_), .. })
		));
	}

	#[tokio::test(start_paused = true)]
	async fn retries_http_500() {
		let endpoint = Scripted::new(vec![
			Err(TransportErrorKind::http_error(500, "Temporary internal error".into())),
			success("\"0x1\""),
		]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
		assert!(matches!(
			response,
			ResponsePacket::Single(Response { payload: ResponsePayload::Success(_), .. })
		));
	}

	#[tokio::test(start_paused = true)]
	async fn does_not_retry_execution_reverted() {
		let endpoint =
			Scripted::new(vec![failure(3, "execution reverted: nope")]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_call", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
		let ResponsePacket::Single(Response { payload: ResponsePayload::Failure(err), .. }) =
			response
		else {
			panic!("expected the original error packet, got {response:?}");
		};
		assert_eq!(err.code, 3);
		assert_eq!(err.message, "execution reverted: nope");
	}

	#[tokio::test(start_paused = true)]
	async fn stops_after_the_backoff_schedule() {
		let endpoint = Scripted::new(vec![
			failure(-32000, "historical state 0x1 is not available"),
			failure(-32000, "historical state 0x1 is not available"),
			failure(-32000, "historical state 0x1 is not available"),
			failure(-32000, "historical state 0x1 is not available"),
			failure(-32000, "historical state 0x1 is not available"),
		]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 5);
		let ResponsePacket::Single(Response { payload: ResponsePayload::Failure(err), .. }) =
			response
		else {
			panic!("expected the last error packet, got {response:?}");
		};
		assert_eq!(err.code, -32000);
	}

	#[tokio::test(start_paused = true)]
	async fn does_not_retry_after_the_budget_is_spent() {
		let endpoint = Scripted::with_delays(vec![(
			Duration::from_secs(61),
			failure(-32000, "historical state 0x1 is not available"),
		)]);
		let calls = endpoint.calls.clone();
		retrying(endpoint)
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
	}

	#[tokio::test(start_paused = true)]
	async fn cuts_a_retry_that_would_exceed_the_budget() {
		let endpoint = Scripted::with_delays(vec![
			(Duration::ZERO, failure(-32000, "historical state 0x1 is not available")),
			(Duration::from_secs(61), failure(-32000, "historical state 0x2 is not available")),
		]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
		let ResponsePacket::Single(Response { payload: ResponsePayload::Failure(err), .. }) =
			response
		else {
			panic!("expected the previous error packet, got {response:?}");
		};
		assert!(err.message.contains("0x1"));
	}

	#[tokio::test(start_paused = true)]
	async fn never_retries_transaction_sends() {
		let endpoint = Scripted::new(vec![Err(TransportErrorKind::http_error(
			500,
			"Temporary internal error".into(),
		))]);
		let calls = endpoint.calls.clone();
		retrying(endpoint)
			.call(request("eth_sendRawTransaction", json!(["0xdeadbeef"])))
			.await
			.unwrap_err();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
	}

	#[tokio::test(start_paused = true)]
	async fn treats_already_known_send_as_success() {
		let endpoint = Scripted::new(vec![failure(-32000, "already known")]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_sendRawTransaction", json!(["0xdeadbeef"])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
		let ResponsePacket::Single(Response { payload: ResponsePayload::Success(value), .. }) =
			response
		else {
			panic!("expected a successful response, got {response:?}");
		};
		let hash = alloy_primitives::keccak256([0xde, 0xad, 0xbe, 0xef]);
		assert_eq!(value.get(), format!("\"{hash}\""));
	}

	#[tokio::test(start_paused = true)]
	async fn treats_already_known_transport_error_as_success() {
		let err = alloy_json_rpc::ErrorPayload {
			code: -32000,
			message: "transaction already known".into(),
			data: None,
		};
		let endpoint = Scripted::new(vec![Err(TransportError::ErrorResp(err))]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_sendRawTransaction", json!(["0xdeadbeef"])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
		assert!(matches!(
			response,
			ResponsePacket::Single(Response { payload: ResponsePayload::Success(_), .. })
		));
	}

	#[tokio::test(start_paused = true)]
	async fn retries_a_null_get_proof() {
		let endpoint = Scripted::new(vec![success("null"), success("\"0x1\"")]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
		let ResponsePacket::Single(Response { payload: ResponsePayload::Success(value), .. }) =
			response
		else {
			panic!("expected a successful response, got {response:?}");
		};
		assert_eq!(value.get(), "\"0x1\"");
	}

	#[tokio::test(start_paused = true)]
	async fn does_not_retry_a_null_receipt() {
		let endpoint = Scripted::new(vec![success("null")]);
		let calls = endpoint.calls.clone();
		let response = retrying(endpoint)
			.call(request("eth_getTransactionReceipt", json!([])))
			.await
			.unwrap();

		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
		assert!(matches!(
			response,
			ResponsePacket::Single(Response { payload: ResponsePayload::Success(_), .. })
		));
	}

	#[tokio::test(start_paused = true)]
	async fn retries_outside_the_fallback_service() {
		let first = Scripted::with_delays(vec![
			(Duration::ZERO, failure(-32000, "historical state 0x1 is not available")),
			(Duration::from_millis(10), success("\"0x1\"")),
		]);
		let second = Scripted::with_delays(vec![
			(Duration::ZERO, failure(-32000, "historical state 0x1 is not available")),
			(Duration::from_millis(10), success("\"0x2\"")),
		]);
		let first_calls = first.calls.clone();
		let second_calls = second.calls.clone();
		let response = RetryLayer
			.layer(fallback_service(vec![first, second]))
			.call(request("eth_getProof", json!([])))
			.await
			.unwrap();

		// Every retry is a fresh round over both endpoints.
		assert_eq!(first_calls.load(std::sync::atomic::Ordering::SeqCst), 2);
		assert_eq!(second_calls.load(std::sync::atomic::Ordering::SeqCst), 2);
		assert!(matches!(
			response,
			ResponsePacket::Single(Response { payload: ResponsePayload::Success(_), .. })
		));
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
