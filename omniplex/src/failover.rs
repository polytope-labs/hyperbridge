//! Failover between a chain's RPC URLs. A request goes to the RPC that answered last, then to
//! each other one in turn while it fails, as SubQuery failed over between endpoints. There is
//! no failing back: the RPC that answers stays first until it fails. An RPC that only refuses a
//! request, such as a method it lacks, stays first for the requests it does serve, unless it
//! refused with an HTTP error.

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

use crate::errors::{
	error_answer, is_answer, is_failed_execution, is_refusal, lacks_method, not_served,
};

/** The method that runs a contract. Its error answer is the call's own failure. */
const CALL: &str = "eth_call";

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
 * An alloy transport over a chain's HTTP RPCs. A result is the chain's answer, the same from
 * any RPC, so it is final. So is an `eth_call`'s error answer, unless the RPC says it can't
 * serve the call or lacks the method: the call ran and failed, in whichever words that client
 * has for it. Sent with an HTTP error, only a failed execution in known words is final, as the
 * body may be the RPC's own complaint, such as a revoked key.
 *
 * Anything else moves the request to the next RPC: no answer, an HTTP error, an answer that the
 * RPC can't serve the request, such as a rate limit or a block it doesn't have, or a refusal of
 * another method, such as one the RPC lacks or its plan leaves out.
 *
 * When no RPC serves the request, the caller gets the outcome a retry is likeliest to change:
 * the first can't-serve answer, else an HTTP error that isn't a refusal, else an RPC that
 * didn't answer. It gets a refusal only when every RPC refused, and a missing method's answer
 * only when every RPC lacks the method, as an RPC that failed may serve the request.
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
			let is_call = request.method_names().all(|method| method == CALL);
			let (mut unserved, mut failure, mut refused, mut lacking) = (None, None, None, None);
			// Whether an RPC before this one failed, rather than only refused the request.
			let mut failed_over = false;
			for index in this.failover.order(this.transports.len()) {
				let mut transport = this.transports[index].clone();
				let outcome = transport.call(request.clone()).await;
				let answer = match &outcome {
					Ok(response) => response.as_error().cloned(),
					Err(err) => error_answer(err),
				};
				let settled = match (&outcome, &answer) {
					(Ok(_), None) => true,
					(Ok(_), Some(answer)) =>
						is_call && !not_served(answer) && !lacks_method(answer),
					(Err(err), _) => is_call && is_failed_execution(err),
				};
				if settled {
					if failed_over {
						this.failover.answered(index);
					}
					return outcome;
				}
				// An HTTP error says the RPC itself is unwell or unwilling, whatever its body.
				let refused_only =
					outcome.is_ok() && answer.as_ref().is_some_and(|answer| !not_served(answer));
				failed_over |= !refused_only;
				let (kept, problem) = match &answer {
					Some(answer) if not_served(answer) => (&mut unserved, "couldn't serve it"),
					Some(answer) if lacks_method(answer) => (&mut lacking, "lacks it"),
					Some(_) => (&mut refused, "refused it"),
					None => (&mut failure, "failed"),
				};
				if several {
					let reason = match (&outcome, &answer) {
						(Ok(_), Some(answer)) => answer.message.to_string(),
						(Err(err), _) => err.to_string(),
						(Ok(_), None) => String::new(),
					};
					tracing::warn!(rpc = index, "RPC {problem}, trying the next: {reason}");
				}
				// The first outcome of each kind is kept, unless a later failure says more.
				let says_more =
					|kept: &Result<ResponsePacket, TransportError>| match (kept, &outcome) {
						(Err(kept), Err(err)) => retry_rank(kept) < retry_rank(err),
						_ => false,
					};
				if kept.as_ref().is_none_or(says_more) {
					*kept = Some(outcome);
				}
			}
			unserved
				.or(failure)
				.or(refused)
				.or(lacking)
				.unwrap_or_else(|| Err(TransportErrorKind::custom_str("no RPC URLs")))
		})
	}
}

/**
 * How strongly a failed RPC's error says a retry could still serve the request: most for an
 * HTTP error that isn't a refusal, such as a 503, then for no answer at all, and least for a
 * refusal.
 */
fn retry_rank(err: &TransportError) -> u8 {
	match err {
		err if is_refusal(err) => 0,
		err if is_answer(err) => 2,
		_ => 1,
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
		/** The response's own fields as JSON: its `result`, its `error`, or both. */
		Body(&'static str),
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
			Reply::Body(fields) => {
				let mut response: serde_json::Value = serde_json::from_str(fields).unwrap();
				response["jsonrpc"] = "2.0".into();
				response["id"] = request["id"].clone();
				(200, response)
			},
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

	/** A provider's answer for a method its plan doesn't include. */
	const OFF_PLAN: &str = "eth_blockNumber is not available on the Free tier";

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
		let slowed = MockRpc::start(Reply::Status(429)).await;
		let rejecting =
			MockRpc::start(Reply::Rejected(403, -32005, "request limit exceeded")).await;
		for first in [
			closed_url().await,
			busy.url.clone(),
			limited.url.clone(),
			lagging.url.clone(),
			throttled.url.clone(),
			slowed.url.clone(),
			rejecting.url.clone(),
		] {
			let (provider, transport) = provider_over(&[&first, &live.url]);
			assert_eq!(provider.get_block_number().await.unwrap(), 42, "after {first}");
			// The live RPC answered, so it now goes first.
			assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [1, 0]);
		}
	}

	#[tokio::test]
	async fn an_rpc_that_only_refuses_the_request_stays_first() {
		let live = MockRpc::start(Reply::Result("0x2a")).await;
		let lacking = MockRpc::start(Reply::Error(-32601, LACKS_METHOD)).await;
		let off_plan = MockRpc::start(Reply::Error(-32600, OFF_PLAN)).await;
		for first in [&lacking, &off_plan] {
			let (provider, transport) = provider_over(&[&first.url, &live.url]);
			assert_eq!(provider.get_block_number().await.unwrap(), 42);
			assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [0, 1]);
		}
		// One that refuses with an HTTP error loses first place, whatever the error's body.
		let forbidden = MockRpc::start(Reply::Status(403)).await;
		let unauthorized = MockRpc::start(Reply::Rejected(401, -32000, "invalid key")).await;
		let rejecting = MockRpc::start(Reply::Rejected(403, -32601, LACKS_METHOD)).await;
		for first in [&forbidden, &unauthorized, &rejecting] {
			let (provider, transport) = provider_over(&[&first.url, &live.url]);
			assert_eq!(provider.get_block_number().await.unwrap(), 42);
			assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [1, 0]);
		}
	}

	#[tokio::test]
	async fn an_eth_call_error_is_final_unless_the_rpc_could_not_serve_the_call() {
		let call = |urls: [&str; 2]| {
			let urls = urls.map(String::from);
			async move {
				let (provider, transport) = provider_over(&[&urls[0], &urls[1]]);
				let outcome = provider.call(Default::default()).await;
				(outcome, transport.failover.order(2).collect::<Vec<_>>())
			}
		};
		// A precompile's failure, in words no list of failed executions has.
		let failed = MockRpc::start(Reply::Error(-32000, "invalid input length")).await;
		let live = MockRpc::start(Reply::Result("0x2a")).await;
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let lagging = MockRpc::start(Reply::Error(-32000, "missing trie node")).await;
		let lacking = MockRpc::start(Reply::Error(-32601, LACKS_METHOD)).await;
		let closed = closed_url().await;
		// The call failed, whatever the other RPCs would say.
		for other in [&live.url, &limited.url, &lagging.url, &closed] {
			let (outcome, order) = call([&failed.url, other]).await;
			let err = outcome.unwrap_err();
			assert_eq!(err.as_error_resp().map(|answer| answer.code), Some(-32000), "{err}");
			assert_eq!(order, [0, 1]);
		}
		// After an RPC that couldn't serve the call, the failure is still final, and its RPC
		// goes first.
		for first in [&limited.url, &lagging.url, &closed] {
			let (outcome, order) = call([first, &failed.url]).await;
			let err = outcome.unwrap_err();
			assert!(err.to_string().contains("invalid input length"), "{err}");
			assert_eq!(order, [1, 0]);
		}
		// The same holds for a revert sent with an HTTP error.
		let reverted = MockRpc::start(Reply::Rejected(400, 3, "execution reverted")).await;
		let (outcome, order) = call([&limited.url, &reverted.url]).await;
		assert!(is_failed_execution(&outcome.unwrap_err()));
		assert_eq!(order, [1, 0]);
		// Any other error sent with an HTTP error may be the RPC's own, such as a revoked key.
		let unauthorized = MockRpc::start(Reply::Rejected(401, -32000, "invalid key")).await;
		let (outcome, order) = call([&unauthorized.url, &live.url]).await;
		assert_eq!(outcome.unwrap().to_string(), "0x2a");
		assert_eq!(order, [1, 0]);
		let (outcome, _) = call([&unauthorized.url, &limited.url]).await;
		let err = outcome.unwrap_err();
		assert_eq!(err.as_error_resp().map(|answer| answer.code), Some(-32005), "{err}");
		// An RPC that can't serve the call, or lacks the method, hands it on.
		for first in [&limited.url, &lagging.url, &lacking.url] {
			let (outcome, _) = call([first, &live.url]).await;
			assert_eq!(outcome.unwrap().to_string(), "0x2a");
		}
	}

	#[tokio::test]
	async fn with_no_rpc_serving_it_the_answer_beats_no_answer() {
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let (provider, _) = provider_over(&[&closed_url().await, &limited.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(err.as_error_resp().is_some_and(|answer| answer.code == -32005), "{err}");
		// An HTTP error is an answer too, and says more than a dead RPC tried before it.
		let busy = MockRpc::start(Reply::Status(503)).await;
		let (provider, _) = provider_over(&[&closed_url().await, &busy.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(is_answer(&err), "{err}");
	}

	#[tokio::test]
	async fn a_refusal_is_the_answer_only_when_every_rpc_refuses() {
		let code = |err: &TransportError| err.as_error_resp().map(|answer| answer.code);
		let off_plan = MockRpc::start(Reply::Error(-32600, OFF_PLAN)).await;
		let forbidden = MockRpc::start(Reply::Status(403)).await;
		let lacking = MockRpc::start(Reply::Error(-32601, LACKS_METHOD)).await;
		let limited = MockRpc::start(Reply::Error(-32005, "daily request limit exceeded")).await;
		let busy = MockRpc::start(Reply::Status(503)).await;
		let closed = closed_url().await;
		let failure = |urls: [&str; 2]| {
			let urls = urls.map(String::from);
			async move {
				let (provider, transport) = provider_over(&[&urls[0], &urls[1]]);
				let err = provider.get_block_number().await.unwrap_err();
				// No RPC served the request, so none takes first place.
				assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [0, 1]);
				err
			}
		};
		// An RPC that is limited, busy or down may serve the request, in either order.
		for refusing in [&off_plan.url, &forbidden.url] {
			for urls in [[refusing, &limited.url], [&limited.url, refusing]] {
				let err = failure(urls.map(String::as_str)).await;
				assert_eq!(code(&err), Some(-32005), "{err}");
			}
			for urls in [[refusing, &busy.url], [&busy.url, refusing]] {
				let err = failure(urls.map(String::as_str)).await;
				assert!(is_answer(&err) && !is_refusal(&err), "{err}");
			}
			for urls in [[refusing, &closed], [&closed, refusing]] {
				let err = failure(urls.map(String::as_str)).await;
				assert!(!is_answer(&err), "{err}");
			}
		}
		// With every RPC refusing, the refusal is the answer, ahead of a missing method.
		for urls in [
			[&off_plan.url, &off_plan.url],
			[&off_plan.url, &lacking.url],
			[&lacking.url, &off_plan.url],
		] {
			let err = failure(urls.map(String::as_str)).await;
			assert_eq!(code(&err), Some(-32600), "{err}");
		}
		// A plain HTTP refusal is one too.
		for urls in [
			[&forbidden.url, &lacking.url],
			[&forbidden.url, &off_plan.url],
			[&off_plan.url, &forbidden.url],
		] {
			let err = failure(urls.map(String::as_str)).await;
			assert!(is_refusal(&err) && code(&err).is_none(), "{err}");
		}
		// One RPC of three that may serve the request is enough to retry it.
		let (provider, _) = provider_over(&[&off_plan.url, &limited.url, &lacking.url]);
		let err = provider.get_block_number().await.unwrap_err();
		assert_eq!(code(&err), Some(-32005), "{err}");
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
		assert!(error_answer(&err).is_some_and(|answer| lacks_method(&answer)), "{err}");
		let (provider, _) = provider_over(&[&rejecting.url, &closed_url().await]);
		let err = provider.get_block_number().await.unwrap_err();
		assert!(!is_answer(&err), "{err}");
	}
}
