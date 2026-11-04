//! Failover between a chain's RPC URLs. A request goes to the RPC that answered last, then to
//! each other one in turn while it fails to answer, as SubQuery failed over between endpoints.

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
 * An alloy transport over a chain's HTTP RPCs. A JSON-RPC error is an answer, so a request
 * moves to the next RPC only when one doesn't answer or answers with an HTTP error.
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
			let mut last = None;
			for index in this.failover.order(this.transports.len()) {
				let mut transport = this.transports[index].clone();
				match transport.call(request.clone()).await {
					Ok(response) => {
						this.failover.answered(index);
						return Ok(response);
					},
					Err(err) => {
						if this.transports.len() > 1 {
							tracing::warn!(
								rpc = index,
								"RPC didn't answer, trying the next: {err}"
							);
						}
						last = Some(err);
					},
				}
			}
			Err(last.unwrap_or_else(|| TransportErrorKind::custom_str("no RPC URLs")))
		})
	}
}

#[cfg(test)]
pub mod tests {
	use alloy::providers::{Provider, ProviderBuilder};
	use tokio::{
		io::{AsyncReadExt, AsyncWriteExt},
		net::TcpListener,
	};

	use super::*;

	/** An RPC that answers every request with `result`, at the URL returned. */
	pub async fn rpc_answering(result: &'static str) -> (String, tokio::task::JoinHandle<()>) {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let url = format!("http://{}", listener.local_addr().unwrap());
		let server = tokio::spawn(async move {
			loop {
				let Ok((mut socket, _)) = listener.accept().await else { return };
				tokio::spawn(async move {
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
					let answer = serde_json::json!({
						"jsonrpc": "2.0",
						"id": request["id"],
						"result": result,
					})
					.to_string();
					let response = format!(
						"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
						answer.len()
					);
					let _ = socket.write_all(response.as_bytes()).await;
				});
			}
		});
		(url, server)
	}

	#[test]
	fn the_rpc_that_answered_last_goes_first() {
		let failover = Failover::default();
		assert_eq!(failover.order(3).collect::<Vec<_>>(), [0, 1, 2]);
		failover.answered(2);
		assert_eq!(failover.order(3).collect::<Vec<_>>(), [2, 0, 1]);
	}

	#[tokio::test]
	async fn a_request_moves_past_an_rpc_that_is_down() {
		let (live, _server) = rpc_answering("0x2a").await;
		let transports = ["http://127.0.0.1:9", live.as_str()]
			.map(|url| Http::with_client(reqwest::Client::new(), url.parse().unwrap()))
			.to_vec();
		let transport = FailoverTransport::new(transports);
		let provider = ProviderBuilder::new().connect_client(
			alloy::rpc::client::ClientBuilder::default().transport(transport.clone(), false),
		);
		assert_eq!(provider.get_block_number().await.unwrap(), 42);
		// The live RPC answered, so it now goes first.
		assert_eq!(transport.failover.order(2).collect::<Vec<_>>(), [1, 0]);
		assert_eq!(provider.get_block_number().await.unwrap(), 42);
	}
}
