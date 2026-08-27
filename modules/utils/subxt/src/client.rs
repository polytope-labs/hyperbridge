use std::time::Duration;

use anyhow::Context;
use reconnecting_jsonrpsee_ws_client::FixedInterval;
use subxt::{
	backend::rpc::RpcClient,
	ext::subxt_rpcs::client::reconnecting_rpc_client::{
		RpcClient as ReconnectingRpcClient, RpcClientBuilder,
	},
	OnlineClient,
};

/// Builds a subxt client that runs both requests and subscriptions over `rpc_ws`.
pub async fn ws_client<T: subxt::Config>(
	rpc_ws: &str,
	max_rpc_payload_size: u32,
) -> Result<(OnlineClient<T>, RpcClient), anyhow::Error> {
	connect(rpc_ws, None, max_rpc_payload_size).await
}

/// Builds a subxt client for a substrate chain. Given an `rpc_http` url, plain rpc requests are
/// sent over http and the websocket is left carrying subscriptions alone, which is all that
/// extrinsic submission and the block streams actually need. A dropped socket then costs us
/// pending subscriptions instead of every query in flight.
pub async fn connect<T: subxt::Config>(
	rpc_ws: &str,
	rpc_http: Option<&str>,
	max_rpc_payload_size: u32,
) -> Result<(OnlineClient<T>, RpcClient), anyhow::Error> {
	let ws = ws_rpc_client(rpc_ws, max_rpc_payload_size).await?;
	let rpc_client = route_requests_over_http(ws, rpc_http, max_rpc_payload_size)?;
	let client = OnlineClient::<T>::from_rpc_client(rpc_client.clone())
		.await
		.context(format!("Failed to query from substrate rpc: {rpc_ws}"))?;

	Ok((client, rpc_client))
}

async fn ws_rpc_client(
	rpc_ws: &str,
	max_rpc_payload_size: u32,
) -> Result<ReconnectingRpcClient, anyhow::Error> {
	let builder = RpcClientBuilder::new()
		// retry every second
		.retry_policy(FixedInterval::new(Duration::from_secs(1)))
		.max_request_size(max_rpc_payload_size)
		.max_response_size(max_rpc_payload_size);

	#[cfg(feature = "std")]
	let builder = builder.enable_ws_ping(
		reconnecting_jsonrpsee_ws_client::PingConfig::new()
			.ping_interval(Duration::from_secs(6))
			.inactive_limit(Duration::from_secs(30)),
	);

	builder
		.build(rpc_ws)
		.await
		.context(format!("Failed to connect to substrate rpc {rpc_ws}"))
}

#[cfg(feature = "std")]
fn route_requests_over_http(
	ws: ReconnectingRpcClient,
	rpc_http: Option<&str>,
	max_rpc_payload_size: u32,
) -> Result<RpcClient, anyhow::Error> {
	let Some(url) = rpc_http else { return Ok(RpcClient::new(ws)) };

	let http = jsonrpsee::http_client::HttpClientBuilder::default()
		.max_request_size(max_rpc_payload_size)
		.max_response_size(max_rpc_payload_size)
		.build(url)
		.context(format!("Failed to connect to substrate rpc {url}"))?;

	Ok(RpcClient::new(split::Transport { http, ws }))
}

/// The browser build has no http transport available, so the websocket carries everything.
#[cfg(feature = "wasm")]
fn route_requests_over_http(
	ws: ReconnectingRpcClient,
	_rpc_http: Option<&str>,
	_max_rpc_payload_size: u32,
) -> Result<RpcClient, anyhow::Error> {
	Ok(RpcClient::new(ws))
}

#[cfg(feature = "std")]
mod split {
	use super::ReconnectingRpcClient;
	use jsonrpsee::{
		core::{client::ClientT, traits::ToRpcParams},
		http_client::HttpClient,
	};
	use subxt::ext::subxt_rpcs::client::{RawRpcFuture, RawRpcSubscription, RawValue, RpcClientT};

	/// Splits a chain's rpc traffic in two: requests are answered over http, while
	/// subscriptions stay on the websocket because http cannot serve them.
	pub struct Transport {
		pub http: HttpClient,
		pub ws: ReconnectingRpcClient,
	}

	impl RpcClientT for Transport {
		fn request_raw<'a>(
			&'a self,
			method: &'a str,
			params: Option<Box<RawValue>>,
		) -> RawRpcFuture<'a, Box<RawValue>> {
			Box::pin(
				async move { Ok(ClientT::request(&self.http, method, PreEncoded(params)).await?) },
			)
		}

		fn subscribe_raw<'a>(
			&'a self,
			sub: &'a str,
			params: Option<Box<RawValue>>,
			unsub: &'a str,
		) -> RawRpcFuture<'a, RawRpcSubscription> {
			self.ws.subscribe_raw(sub, params, unsub)
		}
	}

	/// Params arrive here already serialized, so hand them to jsonrpsee untouched.
	struct PreEncoded(Option<Box<RawValue>>);

	impl ToRpcParams for PreEncoded {
		fn to_rpc_params(self) -> Result<Option<Box<RawValue>>, json::Error> {
			Ok(self.0)
		}
	}
}
