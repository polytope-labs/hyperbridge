use anyhow::Context;
use futures::{StreamExt, TryStreamExt};
use reconnecting_jsonrpsee_ws_client::{Client, PingConfig, RetryPolicy, SubscriptionId};
use std::{ops::Deref, sync::Arc, time::Duration};
use subxt::{
	error::RpcError,
	rpc::{RawValue, RpcClientT, RpcFuture, RpcSubscription},
	OnlineClient,
};

/// Create a reconnecting jsonrpsee client
pub async fn ws_client<T: subxt::Config>(
	rpc_ws: &str,
	max_rpc_payload_size: u32,
) -> Result<OnlineClient<T>, anyhow::Error> {
	let rpc_ws = rpc_ws.to_owned();
	// retry every second
	let retry_policy = RetryPolicy::fixed(Duration::from_secs(1))
		.with_max_retries(usize::MAX)
		.with_max_delay(Duration::from_secs(10));
	let raw_client = Client::builder()
		.retry_policy(retry_policy)
		.max_request_size(max_rpc_payload_size)
		.max_response_size(max_rpc_payload_size)
		.enable_ws_ping(
			PingConfig::new()
				.ping_interval(Duration::from_secs(6))
				.inactive_limit(Duration::from_secs(30)),
		)
		.build(rpc_ws.clone())
		.await
		.context(format!("Failed to connect to substrate rpc {rpc_ws}"))?;
	let client = OnlineClient::<T>::from_rpc_client(Arc::new(ClientWrapper(raw_client)))
		.await
		.context("Failed to query from substrate rpc: {rpc_ws}")?;

	Ok(client)
}

pub struct ClientWrapper(pub Client);

impl Deref for ClientWrapper {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl RpcClientT for ClientWrapper {
	fn request_raw<'a>(
		&'a self,
		method: &'a str,
		params: Option<Box<RawValue>>,
	) -> RpcFuture<'a, Box<RawValue>> {
		Box::pin(async move {
			let res = self
				.0
				.request_raw(method.to_string(), params)
				.await
				.map_err(|e| RpcError::ClientError(Box::new(e)))?;
			Ok(res)
		})
	}

	fn subscribe_raw<'a>(
		&'a self,
		sub: &'a str,
		params: Option<Box<RawValue>>,
		unsub: &'a str,
	) -> RpcFuture<'a, RpcSubscription> {
		Box::pin(async move {
			let stream = self
				.0
				.subscribe_raw(sub.to_string(), params, unsub.to_string())
				.await
				.map_err(|e| RpcError::ClientError(Box::new(e)))?;

			let id = match stream.id() {
				SubscriptionId::Str(id) => Some(id.clone().into_owned()),
				SubscriptionId::Num(id) => Some(id.to_string()),
			};

			let stream = stream.map_err(|e| RpcError::ClientError(Box::new(e))).boxed();
			Ok(RpcSubscription { stream, id })
		})
	}
}
