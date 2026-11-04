//! Failover between a chain's RPC URLs. A request goes to the RPC that answered last, then to
//! each other one in turn while it fails, as SubQuery failed over between endpoints. There is
//! no failing back: the RPC that answers stays first until it fails.

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

use crate::errors::{is_answer, not_served};

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
 * answer, answers with an HTTP error, or answers that it can't serve the request, such as a
 * rate limit or a block it doesn't have. When no RPC serves it, the caller gets the first
 * answer there was, which says more than an RPC that didn't answer.
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
			let (mut unserved, mut failure) = (None, None::<TransportError>);
			for index in this.failover.order(this.transports.len()) {
				let mut transport = this.transports[index].clone();
				match transport.call(request.clone()).await {
					Ok(response) => match response.as_error().filter(|answer| not_served(answer)) {
						None => {
							this.failover.answered(index);
							return Ok(response);
						},
						Some(answer) => {
							if several {
								let message = &answer.message;
								tracing::warn!(
									rpc = index,
									"RPC couldn't serve it, trying the next: {message}"
								);
							}
							unserved.get_or_insert(response);
						},
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
			match (unserved, failure) {
				(Some(response), _) => Ok(response),
				(None, Some(err)) => Err(err),
				(None, None) => Err(TransportErrorKind::custom_str("no RPC URLs")),
			}
		})
	}
}

#[cfg(test)]
pub mod tests {
	use std::sync::Mutex;

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
		Error(i64, &'static str),
		Status(u16),
		/** No logs for an `eth_getLogs` of at most this many blocks, and a range limit for a
		 * longer one. Anything else gets `0x1`. */
		LogsWithin(u64),
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
			Reply::Error(code, message) => (
				200,
				serde_json::json!({
					"jsonrpc": "2.0",
					"id": request["id"],
					"error": { "code": code, "message": message },
				}),
			),
			Reply::Status(status) => (status, serde_json::json!({})),
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
		};
		let answer = answer.to_string();
		let response = format!(
			"HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
			answer.len()
		);
		let _ = socket.write_all(response.as_bytes()).await;
	}

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
		for first in
			[closed_url().await, busy.url.clone(), limited.url.clone(), lagging.url.clone()]
		{
			let (provider, transport) = provider_over(&[&first, &live.url]);
			assert_eq!(provider.get_block_number().await.unwrap(), 42, "after {first}");
			// The live RPC answered, so it now goes first.
			assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [1, 0]);
		}
	}

	#[tokio::test]
	async fn with_no_rpc_serving_it_the_answer_beats_no_answer() {
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let (provider, _) = provider_over(&[&closed_url().await, &limited.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(err.as_error_resp().is_some_and(|answer| answer.code == -32005), "{err}");
		// A revert sent with an HTTP status is an answer too, and says more than a dead RPC.
		let refused = MockRpc::start(Reply::Status(400)).await;
		let (provider, _) = provider_over(&[&refused.url, &closed_url().await]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(is_answer(&err), "{err}");
	}
}
