//! The hyperclient. Allows clients of hyperbridge manage their in-flight ISMP requests.

pub mod internals;
pub mod providers;
use any_client::AnyClient;
use providers::interface::Client;
pub use subxt_utils::gargantua as runtime;
pub mod any_client;
pub mod types;

pub mod interfaces;

extern crate alloc;
extern crate core;

use crate::types::ClientConfig;

use crate::{
	interfaces::{JsClientConfig, JsPost, JsPostResponse},
	providers::substrate::SubstrateClient,
	types::{MessageStatusWithMetadata, TimeoutStatus},
};
use ethers::{types::H256, utils::keccak256};
use futures::StreamExt;
use ismp::router::{PostRequest, PostResponse};
use subxt_utils::Hyperbridge;
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

#[cfg(any(feature = "testing", test))]
pub mod testing;

pub mod indexing;

#[cfg(test)]
mod tests;

#[wasm_bindgen(typescript_custom_section)]
const ICONFIG: &'static str = r#"
interface IConfig {
    // confuration object for the source chain
    source: IChainConfig;
    // confuration object for the destination chain
    dest: IChainConfig;
    // confuration object for hyperbridge
    hyperbridge: IHyperbridgeConfig;
	// Indexer url
	indexer: string;
}

interface IChainConfig {
    // rpc url of the chain
    rpc_url: string;
    // state machine identifier as a string
    state_machine: string;
    // contract address of the `IsmpHost` on this chain
    host_address: Uint8Array;
    // contract address of the `IHandler` on this chain
    handler_address: Uint8Array;
    // consensus state identifier of this chain on hyperbridge
    consensus_state_id: Uint8Array;
}

interface IHyperbridgeConfig {
    // websocket rpc endpoint for hyperbridge
    rpc_url: string;
}

interface IPostRequest {
    // The source state machine of this request.
    source: string;
    // The destination state machine of this request.
    dest: string;
    // Module Id of the sending module
    from: string;
    // Module ID of the receiving module
    to: string;
    // The nonce of this request on the source chain
    nonce: bigint;
    // Encoded request body.
    data: string;
    // Timestamp which this request expires in seconds.
    timeout_timestamp: bigint;
    // Height at which this request was emitted on the source
    height: bigint;
}

interface IPostResponse {
    // The request that triggered this response.
    post: IPostRequest;
    // The response message.
    response: Uint8Array;
    // Timestamp at which this response expires in seconds.
    timeout_timestamp: bigint;
}

type MessageStatus =  Pending | SourceFinalized | HyperbridgeDelivered | HyperbridgeFinalized | DestinationDelivered | Timeout;

// This transaction is still pending on the source chain
interface Pending {
    kind: "Pending";
}

// This event is emitted on hyperbridge
interface SourceFinalized {
    kind: "SourceFinalized";
}

// This event is emitted on hyperbridge
interface HyperbridgeDelivered {
    kind: "HyperbridgeDelivered";
}

// This event is emitted on the destination chain
interface HyperbridgeFinalized {
    kind: "HyperbridgeFinalized";
}

// This event is emitted on the destination chain
interface DestinationDelivered {
    kind: "DestinationDelivered";
}

// The request has now timed out
interface Timeout {
    kind: "Timeout";
}

// The request has now timed out
interface DestinationFinalized {
    kind: "DestinationFinalized";
}

// The request has now timed out
interface HyperbridgeTimedout {
    kind: "HyperbridgeTimedout";
}


// The request has now timed out
interface HyperbridgeTimedout {
    kind: "HyperbridgeTimedout";
}

// The possible states of an inflight request
type MessageStatusWithMeta =  SourceFinalizedWithMetadata | HyperbridgeDeliveredWithMetadata | HyperbridgeFinalizedWithMetadata | DestinationDeliveredWithMetadata | Timeout | ErrorWithMetadata;

// The possible states of a timed-out request
type TimeoutStatusWithMeta =  DestinationFinalizedWithMetadata | HyperbridgeTimedoutWithMetadata | HyperbridgeFinalizedWithMetadata | TimeoutMessage | ErrorWithMetadata;


// This event is emitted on hyperbridge
interface SourceFinalizedWithMetadata {
    kind: "SourceFinalized";
    // Block height of the source chain that was finalized.
    finalized_height: bigint;
    // The hash of the block where the event was emitted
    block_hash: `0x{string}`;
    // The hash of the extrinsic responsible for the event
    transaction_hash: `0x{string}`;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on hyperbridge
interface HyperbridgeDeliveredWithMetadata {
    kind: "HyperbridgeDelivered";
    // The hash of the block where the event was emitted
    block_hash: `0x{string}`;
    // The hash of the extrinsic responsible for the event
    transaction_hash: `0x{string}`;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on the destination chain
interface HyperbridgeFinalizedWithMetadata {
    kind: "HyperbridgeFinalized";
    // Block height of hyperbridge chain that was finalized.
    finalized_height: bigint;
    // The hash of the block where the event was emitted
    block_hash: `0x{string}`;
    // The hash of the extrinsic responsible for the event
    transaction_hash: `0x{string}`;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on hyperbridge
interface HyperbridgeTimedoutWithMetadata {
    kind: "HyperbridgeTimedout";
    // The hash of the block where the event was emitted
    block_hash: `0x{string}`;
    // The hash of the extrinsic responsible for the event
    transaction_hash: `0x{string}`;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on the destination chain
interface DestinationDeliveredWithMetadata {
    kind: "DestinationDelivered";
    // The hash of the block where the event was emitted
    block_hash: `0x{string}`;
    // The hash of the extrinsic responsible for the event
    transaction_hash: `0x{string}`;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on the destination chain
interface TimeoutMessage {
    kind: "TimeoutMessage";
    // encoded call for HandlerV1.handlePostRequestTimeouts
    calldata: Uint8Array,
}

// This event is emitted on hyperbridge
interface DestinationFinalizedWithMetadata {
    kind: "DestinationFinalized";
    // The hash of the block where the event was emitted
    block_hash: `0x{string}`;
    // The hash of the extrinsic responsible for the event
    transaction_hash: `0x{string}`;
    // The block number where the event was emitted
    block_number: bigint;
}


// An error was encountered in the stream, the stream will come to an end.
interface ErrorWithMetadata {
    kind: "Error";
    // error description
    description: string
}
"#;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(typescript_type = "IConfig")]
	pub type IConfig;

	#[wasm_bindgen(typescript_type = "IPostRequest")]
	pub type IPostRequest;

	#[wasm_bindgen(typescript_type = "IPostResponse")]
	pub type IPostResponse;
}

/// The hyperclient, allows the clients of hyperbridge to manage their in-flight ISMP requests
/// across multiple chains.
#[wasm_bindgen]
#[derive(Clone)]
pub struct HyperClient {
	#[wasm_bindgen(skip)]
	pub source: AnyClient,
	#[wasm_bindgen(skip)]
	pub dest: AnyClient,
	#[wasm_bindgen(skip)]
	pub hyperbridge: SubstrateClient<Hyperbridge>,
	#[wasm_bindgen(skip)]
	pub indexer: Option<String>,
}

impl HyperClient {
	/// Initialize the Hyperclient
	pub async fn new(config: ClientConfig) -> Result<Self, anyhow::Error> {
		tracing::info!("Connecting to source");
		let source = config.source_chain().await?;

		tracing::info!("Connecting to dest");
		let dest = config.dest_chain().await?;

		tracing::info!("Connecting to hyperbridge");
		let hyperbridge = config.hyperbridge_client().await?;

		tracing::info!("Connected to hyperbridge");
		Ok(Self { source, dest, hyperbridge, indexer: config.indexer.clone() })
	}
}

#[wasm_bindgen]
impl HyperClient {
	/// Initialize the hyperclient
	pub async fn init(config: IConfig) -> Result<HyperClient, JsError> {
		let lambda = || async move {
			let config = serde_wasm_bindgen::from_value::<JsClientConfig>(config.into()).unwrap();
			let config: ClientConfig = config.try_into()?;

			HyperClient::new(config).await
		};

		lambda().await.map_err(|err: anyhow::Error| {
			JsError::new(&format!("Could not create hyperclient {err:?}"))
		})
	}

	/// Queries the status of a request and returns `MessageStatusWithMetadata`
	pub async fn query_request_status(&self, request: IPostRequest) -> Result<JsValue, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPost>(request.into()).unwrap();
			let post: PostRequest = post.try_into()?;
			let status = internals::query_request_status_internal(&self, post).await?;
			Ok(serde_wasm_bindgen::to_value(&status).expect("Infallible"))
		};

		lambda().await.map_err(|err: anyhow::Error| {
			JsError::new(&format!(
				"Failed to query request status for {:?}->{:?}: {err:?}",
				self.source.state_machine_id().state_id,
				self.dest.state_machine_id().state_id,
			))
		})
	}

	/// Accepts a post response and returns a `MessageStatusWithMetadata`
	pub async fn query_response_status(&self, response: IPostResponse) -> Result<JsValue, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPostResponse>(response.into()).unwrap();
			let response: PostResponse = post.try_into()?;
			let status = internals::query_response_status_internal(&self, response).await?;
			Ok(serde_wasm_bindgen::to_value(&status).expect("Infallible"))
		};

		lambda().await.map_err(|err: anyhow::Error| {
			JsError::new(&format!(
				"Failed to query response status for {:?}->{:?}: {err:?}",
				self.source.state_machine_id().state_id,
				self.dest.state_machine_id().state_id,
			))
		})
	}

	/// Return the status of a post request as a `ReadableStream` that yields
	/// `MessageStatusWithMeta`
	pub async fn request_status_stream(
		&self,
		request: IPostRequest,
	) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPost>(request.into()).unwrap();
			let height = post.height;
			let post: PostRequest = post.try_into()?;

			// Obtaining the request stream and the timeout stream
			let timed_out =
				internals::request_timeout_stream(post.timeout_timestamp, self.source.clone())
					.await;

			let request_status = internals::request_status_stream(&self, post, height).await;

			let stream = futures::stream::select(request_status, timed_out).map(|res| {
				res.map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible"))
					.map_err(|e| {
						serde_wasm_bindgen::to_value(&MessageStatusWithMetadata::Error {
							description: alloc::format!("{e:?}"),
						})
						.expect("Infallible")
					})
			});

			// Wrapping the main stream in a readable stream
			let js_stream = ReadableStream::from_stream(stream);

			Ok(js_stream.into_raw())
		};

		lambda().await.map_err(|err: anyhow::Error| {
			JsError::new(&format!("Failed to create request status stream: {err:?}"))
		})
	}

	/// Given a post request that has timed out returns a `ReadableStream` that yields a
	/// `TimeoutStatus` This function will not check if the request has timed out, only call it
	/// when you receive a `MesssageStatus::TimeOut` from `query_request_status` or
	/// `request_status_stream`. The stream ends when once it yields a `TimeoutMessage`
	pub async fn timeout_post_request(
		&self,
		request: IPostRequest,
	) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPost>(request.into()).unwrap();
			let post: PostRequest = post.try_into()?;

			let stream = internals::timeout_request_stream(&self, post).await?.map(|value| {
				value
					.map(|status| serde_wasm_bindgen::to_value(&status).expect("Infallible"))
					.map_err(|e| {
						serde_wasm_bindgen::to_value(&TimeoutStatus::Error {
							description: alloc::format!("{e:?}"),
						})
						.expect("Infallible")
					})
			});

			let js_stream = ReadableStream::from_stream(stream);
			Ok(js_stream.into_raw())
		};

		lambda().await.map_err(|err: anyhow::Error| {
			JsError::new(&format!("Failed to create post request timeout stream: {err:?}"))
		})
	}

	pub fn get_indexer_url(&self) -> Option<String> {
		self.indexer.clone()
	}
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	// print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
	// This is not needed for tracing_wasm to work, but it is a common tool for getting proper error
	// line numbers for panics.
	console_error_panic_hook::set_once();

	use tracing_subscriber_wasm::MakeConsoleWriter;

	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.with_writer(
			// To avoide trace events in the browser from showing their
			// JS backtrace, which is very annoying, in my opinion
			MakeConsoleWriter::default().map_trace_level_to(tracing::Level::INFO),
		)
		// For some reason, if we don't do this in the browser, we get
		// a runtime error.
		.without_time()
		.init();

	Ok(())
}

#[derive(Clone, Default)]
pub struct Keccak256;

impl ismp::messaging::Keccak256 for Keccak256 {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		keccak256(bytes).into()
	}
}
