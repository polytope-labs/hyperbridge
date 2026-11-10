//! Failover between a chain's RPC URLs. A request goes to the RPC that answered last, then to
//! each other one in turn while it fails, as SubQuery failed over between endpoints. There is
//! no failing back: the RPC that answers stays first until it fails. An RPC that only lacks a
//! method stays first for the requests it does serve.

use std::{
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	task::{Context, Poll},
};

use alloy::{
	rpc::json_rpc::{RequestPacket, ResponsePacket},
	transports::{
		http::{reqwest, Http},
		TransportError, TransportErrorKind, TransportFut,
	},
};
use tower_service::Service;

use crate::errors::{is_answer, is_failed_execution, is_missing_method, lacks_method, not_served};

/** Which of a chain's RPCs to try first: the one that answered last. */
#[derive(Debug, Default)]
pub struct Failover {
	current: AtomicUsize,
}

impl Failover {
	/** The indices of `count` RPCs in the order to try them. */
	pub fn order(&self, count: usize) -> impl Iterator<Item = usize> {
		let start = self.current.load(Ordering::Relaxed);
		(0..count).map(move |offset| (start + offset) % count)
	}

	/** Records that RPC `index` answered, so it is tried first from now on. */
	pub fn answered(&self, index: usize) {
		self.current.store(index, Ordering::Relaxed);
	}
}

/**
 * An alloy transport over a chain's HTTP RPCs. A request moves to the next RPC when one doesn't
 * answer, answers with an HTTP error, answers that it can't serve the request, such as a rate
 * limit or a block it doesn't have, or answers that it lacks the method. A revert is final,
 * even sent with an HTTP error. When no RPC serves the request, the caller gets the first
 * can't-serve answer, else the first HTTP error, else the first failure: an answer says more
 * than an RPC that didn't answer. It gets a missing method's answer, with or without an HTTP
 * error, only when every RPC gave one, as an RPC that failed may have the method.
 */
#[derive(Clone)]
pub struct FailoverTransport {
	transports: Arc<Vec<Http<reqwest::Client>>>,
	failover: Arc<Failover>,
}

impl FailoverTransport {
	pub fn new(transports: Vec<Http<reqwest::Client>>) -> Self {
		Self { transports: Arc::new(transports), failover: Default::default() }
	}
}

impl Service<RequestPacket> for FailoverTransport {
	type Response = ResponsePacket;
	type Error = TransportError;
	type Future = TransportFut<'static>;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, request: RequestPacket) -> Self::Future {
		let this = self.clone();
		Box::pin(async move {
			let several = this.transports.len() > 1;
			let (mut unserved, mut lacking, mut failure) = (None, None, None::<TransportError>);
			for index in this.failover.order(this.transports.len()) {
				let mut transport = this.transports[index].clone();
				// Whether an RPC before this one failed, rather than only lacked the method.
				let failed_over = unserved.is_some() || failure.is_some();
				match transport.call(request.clone()).await {
					Ok(response) => {
						let (kept, message) = match response.as_error() {
							Some(answer) if lacks_method(answer) =>
								(&mut lacking, answer.message.to_string()),
							Some(answer) if not_served(answer) =>
								(&mut unserved, answer.message.to_string()),
							_ => {
								if failed_over {
									this.failover.answered(index);
								}
								return Ok(response);
							},
						};
						if several {
							tracing::warn!(
								rpc = index,
								"RPC couldn't serve it, trying the next: {message}"
							);
						}
						kept.get_or_insert(Ok(response));
					},
					Err(err) if is_failed_execution(&err) => {
						if failed_over {
							this.failover.answered(index);
						}
						return Err(err);
					},
					Err(err) if is_missing_method(&err) => {
						if several {
							tracing::warn!(rpc = index, "RPC lacks it, trying the next: {err}");
						}
						lacking.get_or_insert(Err(err));
					},
					Err(err) => {
						if several {
							tracing::warn!(rpc = index, "RPC failed, trying the next: {err}");
						}
						if failure.as_ref().is_none_or(|kept| !is_answer(kept) && is_answer(&err)) {
							failure = Some(err);
						}
					},
				}
			}
			match (unserved, failure, lacking) {
				(Some(answer), _, _) => answer,
				(None, Some(err), _) => Err(err),
				(None, None, Some(answer)) => answer,
				(None, None, None) => Err(TransportErrorKind::custom_str("no RPC URLs")),
			}
		})
	}
}

#[cfg(test)]
pub mod tests {
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Mutex,
	};

	use alloy::providers::{Provider, ProviderBuilder};
	use tokio::{
		io::{AsyncReadExt, AsyncWriteExt},
		net::TcpListener,
	};

	use super::*;

	/** How a mock RPC answers every request. */
	#[derive(Debug, Clone)]
	pub enum Reply {
		Result(&'static str),
		/** A `null` result, as for a block the node doesn't have. */
		Null,
		Error(i64, &'static str),
		Status(u16),
		/** A JSON-RPC error sent with an HTTP error status. */
		Rejected(u16, i64, &'static str),
		/** No logs for an `eth_getLogs` of at most this many blocks, and a range limit for a
		 * longer one. Anything else gets `0x1`. */
		LogsWithin(u64),
		/** For `eth_getLogs` by block hash, `log`, or `hash_error` when set. By range, no logs
		 * while `misses` lasts, as from a node behind the tip, then `log`. For
		 * `eth_getBlockByNumber`, `canonical`, the block now at the tip's height. */
		TipLogs {
			log: &'static str,
			misses: Arc<AtomicUsize>,
			hash_error: Option<(i64, &'static str)>,
			canonical: &'static str,
		},
		/** For `eth_getBlockByNumber`, no block while `nulls` lasts, as from a node behind it,
		 * then `block`. No logs for an `eth_getLogs`. */
		Block {
			block: &'static str,
			nulls: Arc<AtomicUsize>,
		},
	}

	/** An RPC on a local port, answering as its `reply` says at the time. */
	pub struct MockRpc {
		pub url: String,
		pub reply: Arc<Mutex<Reply>>,
		pub server: tokio::task::JoinHandle<()>,
	}

	impl MockRpc {
		pub async fn start(reply: Reply) -> Self {
			let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
			let url = format!("http://{}", listener.local_addr().unwrap());
			let reply = Arc::new(Mutex::new(reply));
			let current = reply.clone();
			let server = tokio::spawn(async move {
				loop {
					let Ok((socket, _)) = listener.accept().await else { return };
					let reply = current.lock().unwrap().clone();
					tokio::spawn(answer(socket, reply));
				}
			});
			Self { url, reply, server }
		}

		pub fn set(&self, reply: Reply) {
			*self.reply.lock().unwrap() = reply;
		}
	}

	async fn answer(mut socket: tokio::net::TcpStream, reply: Reply) {
		let mut request = Vec::new();
		let mut buffer = [0u8; 4096];
		// Read the headers, then as much body as they announce.
		let body = loop {
			let Ok(read) = socket.read(&mut buffer).await else { return };
			if read == 0 {
				return;
			}
			request.extend_from_slice(&buffer[..read]);
			let text = String::from_utf8_lossy(&request).to_string();
			let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
			let length = head
				.lines()
				.find_map(|line| {
					let (name, value) = line.split_once(':')?;
					name.eq_ignore_ascii_case("content-length")
						.then(|| value.trim().parse::<usize>().ok())?
				})
				.unwrap_or(0);
			if body.len() >= length {
				break body.to_string();
			}
		};
		let request: serde_json::Value = serde_json::from_str(&body).unwrap();
		let (status, answer) = match reply {
			Reply::Result(result) => (
				200,
				serde_json::json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }),
			),
			Reply::Null =>
				(200, serde_json::json!({ "jsonrpc": "2.0", "id": request["id"], "result": null })),
			Reply::Error(code, message) => (
				200,
				serde_json::json!({
					"jsonrpc": "2.0",
					"id": request["id"],
					"error": { "code": code, "message": message },
				}),
			),
			Reply::Status(status) => (status, serde_json::json!({})),
			Reply::Rejected(status, code, message) => (
				status,
				serde_json::json!({
					"jsonrpc": "2.0",
					"id": request["id"],
					"error": { "code": code, "message": message },
				}),
			),
			Reply::LogsWithin(most) => {
				let block = |field: &str| {
					let hex = request["params"][0][field].as_str().unwrap_or("0x0");
					u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0)
				};
				let id = &request["id"];
				let answer = match request["method"].as_str() {
					Some("eth_getLogs") if block("toBlock") - block("fromBlock") + 1 > most =>
						serde_json::json!({
							"jsonrpc": "2.0",
							"id": id,
							"error": { "code": -32000, "message": "block range too large" },
						}),
					Some("eth_getLogs") =>
						serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": [] }),
					_ => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": "0x1" }),
				};
				(200, answer)
			},
			Reply::TipLogs { log, misses, hash_error, canonical } => {
				let (id, log) =
					(&request["id"], serde_json::from_str::<serde_json::Value>(log).unwrap());
				let by_hash = request["params"][0].get("blockHash").is_some();
				let result =
					|result| serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
				let answer = match request["method"].as_str() {
					Some("eth_getLogs") if by_hash => match hash_error {
						Some((code, message)) => serde_json::json!({
							"jsonrpc": "2.0",
							"id": id,
							"error": { "code": code, "message": message },
						}),
						None => result(serde_json::json!([log])),
					},
					Some("eth_getLogs") => {
						let missed = misses
							.try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1));
						result(if missed.is_ok() {
							serde_json::json!([])
						} else {
							serde_json::json!([log])
						})
					},
					Some("eth_getBlockByNumber") =>
						result(serde_json::from_str::<serde_json::Value>(canonical).unwrap()),
					_ => result(serde_json::json!("0x1")),
				};
				(200, answer)
			},
			Reply::Block { block, nulls } => {
				let result = match request["method"].as_str() {
					Some("eth_getBlockByNumber") => {
						let missed = nulls
							.try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1));
						match missed {
							Ok(_) => serde_json::Value::Null,
							Err(_) => serde_json::from_str(block).unwrap(),
						}
					},
					Some("eth_getLogs") => serde_json::json!([]),
					_ => serde_json::json!("0x1"),
				};
				(
					200,
					serde_json::json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }),
				)
			},
		};
		let answer = answer.to_string();
		let response = format!(
			"HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
			answer.len()
		);
		let _ = socket.write_all(response.as_bytes()).await;
	}

	/** geth's answer for a method it doesn't serve. */
	const LACKS_METHOD: &str = "the method eth_blockNumber does not exist/is not available";

	/** A local URL nothing listens on. */
	pub async fn closed_url() -> String {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		format!("http://{}", listener.local_addr().unwrap())
	}

	fn provider_over(urls: &[&str]) -> (impl Provider, FailoverTransport) {
		let transports = urls
			.iter()
			.map(|url| Http::with_client(reqwest::Client::new(), url.parse().unwrap()))
			.collect();
		let transport = FailoverTransport::new(transports);
		let client =
			alloy::rpc::client::ClientBuilder::default().transport(transport.clone(), false);
		(ProviderBuilder::new().connect_client(client), transport)
	}

	#[test]
	fn the_rpc_that_answered_last_goes_first() {
		let failover = Failover::default();
		assert_eq!(failover.order(3).collect::<Vec<_>>(), [0, 1, 2]);
		failover.answered(2);
		assert_eq!(failover.order(3).collect::<Vec<_>>(), [2, 0, 1]);
	}

	#[tokio::test]
	async fn a_request_moves_past_an_rpc_that_fails_or_cannot_serve_it() {
		let live = MockRpc::start(Reply::Result("0x2a")).await;
		let busy = MockRpc::start(Reply::Status(503)).await;
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let lagging = MockRpc::start(Reply::Error(-32000, "header not found")).await;
		let throttled = MockRpc::start(Reply::Error(-32000, "Too Many Requests")).await;
		for first in [
			closed_url().await,
			busy.url.clone(),
			limited.url.clone(),
			lagging.url.clone(),
			throttled.url.clone(),
		] {
			let (provider, transport) = provider_over(&[&first, &live.url]);
			assert_eq!(provider.get_block_number().await.unwrap(), 42, "after {first}");
			// The live RPC answered, so it now goes first.
			assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [1, 0]);
		}
	}

	#[tokio::test]
	async fn an_rpc_that_only_lacks_the_method_stays_first() {
		let live = MockRpc::start(Reply::Result("0x2a")).await;
		let lacking = MockRpc::start(Reply::Error(-32601, LACKS_METHOD)).await;
		let rejecting = MockRpc::start(Reply::Rejected(403, -32601, LACKS_METHOD)).await;
		for first in [&lacking, &rejecting] {
			let (provider, transport) = provider_over(&[&first.url, &live.url]);
			assert_eq!(provider.get_block_number().await.unwrap(), 42);
			assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [0, 1]);
		}
	}

	#[tokio::test]
	async fn with_no_rpc_serving_it_the_answer_beats_no_answer() {
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let (provider, _) = provider_over(&[&closed_url().await, &limited.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(err.as_error_resp().is_some_and(|answer| answer.code == -32005), "{err}");
		// An HTTP error is an answer too, and says more than a dead RPC tried before it.
		let refused = MockRpc::start(Reply::Status(400)).await;
		let (provider, _) = provider_over(&[&closed_url().await, &refused.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(is_answer(&err), "{err}");
		// A revert is final, even with an HTTP error, so it beats an earlier RPC's rate limit.
		let reverted = MockRpc::start(Reply::Rejected(400, 3, "execution reverted")).await;
		let (provider, _) = provider_over(&[&limited.url, &reverted.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(is_failed_execution(&err), "{err}");
	}

	#[tokio::test]
	async fn a_missing_method_is_the_answer_only_when_every_rpc_lacks_it() {
		let code = |err: &TransportError| err.as_error_resp().map(|answer| answer.code);
		let lacking = MockRpc::start(Reply::Error(-32601, LACKS_METHOD)).await;
		let (provider, _) = provider_over(&[&lacking.url, &lacking.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert_eq!(code(&err), Some(-32601), "{err}");
		// An RPC that is limited or down may have the method, so its failure is the answer.
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let (provider, _) = provider_over(&[&lacking.url, &limited.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert_eq!(code(&err), Some(-32005), "{err}");
		let (provider, _) = provider_over(&[&lacking.url, &closed_url().await]);
		let err = provider.get_block_number().await.unwrap_err();
		assert_eq!(code(&err), None, "{err}");
		// The same holds when the missing method comes with an HTTP error.
		let rejecting = MockRpc::start(Reply::Rejected(403, -32601, LACKS_METHOD)).await;
		let (provider, _) = provider_over(&[&rejecting.url, &rejecting.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(is_missing_method(&err), "{err}");
		let (provider, _) = provider_over(&[&rejecting.url, &closed_url().await]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(!is_answer(&err), "{err}");
	}
}
