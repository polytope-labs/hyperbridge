use std::time::Duration;

use anyhow::Context;
use reconnecting_jsonrpsee_ws_client::FixedInterval;
use subxt::{
	backend::rpc::RpcClient, ext::subxt_rpcs::client::reconnecting_rpc_client::RpcClientBuilder,
	OnlineClient,
};

#[cfg(feature = "std")]
pub async fn ws_client<T: subxt::Config>(
	rpc_ws: &str,
	max_rpc_payload_size: u32,
) -> Result<(OnlineClient<T>, RpcClient), anyhow::Error> {
	let rpc_client = RpcClientBuilder::new()
		// retry every second
		.retry_policy(FixedInterval::new(Duration::from_secs(1)))
		.max_request_size(max_rpc_payload_size)
		.max_response_size(max_rpc_payload_size)
		.enable_ws_ping(
			reconnecting_jsonrpsee_ws_client::PingConfig::new()
				.ping_interval(Duration::from_secs(6))
				.inactive_limit(Duration::from_secs(30)),
		)
		.build(rpc_ws)
		.await
		.context(format!("Failed to connect to substrate rpc {rpc_ws}"))?;

	let client = OnlineClient::<T>::from_rpc_client(rpc_client.clone())
		.await
		.context(format!("Failed to query from substrate rpc: {rpc_ws}"))?;

	Ok((client, RpcClient::new(rpc_client)))
}

#[cfg(feature = "wasm")]
pub async fn ws_client<T: subxt::Config>(
	rpc_ws: &str,
	max_rpc_payload_size: u32,
) -> Result<(OnlineClient<T>, RpcClient), anyhow::Error> {
	let rpc_client = RpcClientBuilder::new()
		// retry every second
		.retry_policy(FixedInterval::new(Duration::from_secs(1)))
		.max_request_size(max_rpc_payload_size)
		.max_response_size(max_rpc_payload_size)
		.build(rpc_ws)
		.await
		.context(format!("Failed to connect to substrate rpc {rpc_ws}"))?;

	let client = OnlineClient::<T>::from_rpc_client(rpc_client.clone())
		.await
		.context(format!("Failed to query from substrate rpc: {rpc_ws}"))?;

	Ok((client, RpcClient::new(rpc_client)))
}
