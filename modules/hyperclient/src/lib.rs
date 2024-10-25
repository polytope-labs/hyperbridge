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

//! The hyperclient. Allows clients of hyperbridge manage their in-flight ISMP requests.

pub mod internals;
pub mod providers;
use any_client::AnyClient;
use ismp::messaging::{hash_post_response, hash_request};
use providers::interface::Client;
pub use subxt_utils::gargantua as runtime;
pub mod any_client;
pub mod types;

pub mod interfaces;

extern crate alloc;
extern crate core;

use crate::types::{ClientConfig, MessageStatusStreamState, TimeoutStreamState};

use crate::{
	interfaces::{JsClientConfig, JsGet, JsPost, JsPostResponse},
	providers::substrate::SubstrateClient,
	types::{MessageStatusWithMetadata, TimeoutStatus},
};
use ethers::{types::H256, utils::keccak256};
use futures::StreamExt;
use ismp::router::{GetRequest, PostRequest, PostResponse, Request};
use subxt_utils::Hyperbridge;
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

#[cfg(any(feature = "testing", test))]
pub mod testing;

pub mod indexing;

#[cfg(test)]
mod tests;

/// The hyperclient, allows the clients of hyperbridge to manage their in-flight ISMP requests
/// across multiple chains.
#[wasm_bindgen]
#[derive(Clone)]
pub struct HyperClient {
	/// Internal client for the source chain
	#[wasm_bindgen(skip)]
	pub source: AnyClient,
	#[wasm_bindgen(skip)]
	/// Internal client for the destination chain
	pub dest: AnyClient,
	#[wasm_bindgen(skip)]
	/// Internal client for Hyperbridge
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
	pub async fn init(config: JsValue) -> Result<HyperClient, JsError> {
		let lambda = || async move {
			let config = serde_wasm_bindgen::from_value::<JsClientConfig>(config).unwrap();
			let config: ClientConfig = config.try_into()?;

			if config.tracing {
				use tracing_subscriber_wasm::MakeConsoleWriter;

				tracing_subscriber::fmt()
					.with_max_level(tracing::Level::TRACE)
					.with_writer(
						// To avoide trace events in the browser from showing their
						// JS backtrace, which is very annoying, in my opinion
						MakeConsoleWriter::default().map_trace_level_to(tracing::Level::INFO),
					)
					// For some reason, if we don't do this in the browser, we get
					// a runtime error.
					.without_time()
					.init();
			}

			HyperClient::new(config).await
		};

		lambda().await.map_err(|err: anyhow::Error| {
			JsError::new(&format!("Could not create hyperclient: {err:?}"))
		})
	}

	/// Returns the commitment for the provided POST request
	pub fn post_request_commitment(post: JsValue) -> Result<JsValue, JsError> {
		let lambda = || {
			let post = serde_wasm_bindgen::from_value::<JsPost>(post.into()).unwrap();
			let post: PostRequest = post.try_into()?;
			let commitment = hash_request::<Keccak256>(&Request::Post(post));
			Ok(serde_wasm_bindgen::to_value(&commitment).expect("Infallible"))
		};

		lambda().map_err(|err: anyhow::Error| {
			JsError::new(&format!("Failed to derive request commitment: {err:?}",))
		})
	}

	/// Returns the commitment for the provided GET request
	pub fn get_request_commitment(get: JsValue) -> Result<JsValue, JsError> {
		let lambda = || {
			let get = serde_wasm_bindgen::from_value::<JsGet>(get.into()).unwrap();
			let get: GetRequest = get.try_into()?;
			let commitment = hash_request::<Keccak256>(&Request::Get(get));
			Ok(serde_wasm_bindgen::to_value(&commitment).expect("Infallible"))
		};

		lambda().map_err(|err: anyhow::Error| {
			JsError::new(&format!("Failed to derive request commitment: {err:?}",))
		})
	}

	/// Returns the commitment for the provided POST response
	pub fn post_response_commitment(response: JsValue) -> Result<JsValue, JsError> {
		let lambda = || {
			let response =
				serde_wasm_bindgen::from_value::<JsPostResponse>(response.into()).unwrap();
			let response: PostResponse = response.try_into()?;
			let commitment = hash_post_response::<Keccak256>(&response);
			Ok(serde_wasm_bindgen::to_value(&commitment).expect("Infallible"))
		};

		lambda().map_err(|err: anyhow::Error| {
			JsError::new(&format!("Failed to derive request commitment: {err:?}",))
		})
	}

	/// Queries the status of a request and returns `MessageStatusWithMetadata`
	pub async fn query_post_request_status(&self, request: JsValue) -> Result<JsValue, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPost>(request.into()).unwrap();
			let post: PostRequest = post.try_into()?;
			let status = internals::query_post_request_status_internal(&self, post).await?;
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

	/// Queries the status of a request and returns `MessageStatusWithMetadata`
	pub async fn query_get_request_status(&self, request: JsValue) -> Result<JsValue, JsError> {
		let lambda = || async move {
			let get = serde_wasm_bindgen::from_value::<JsGet>(request.into()).unwrap();
			let get: GetRequest = get.try_into()?;
			let status = internals::query_get_request_status(&self, get).await?;
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
	pub async fn query_post_response_status(&self, response: JsValue) -> Result<JsValue, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPostResponse>(response).unwrap();
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
	pub async fn post_request_status_stream(
		&self,
		request: JsValue,
		initial_state: JsValue,
	) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPost>(request).unwrap();
			let state =
				serde_wasm_bindgen::from_value::<MessageStatusStreamState>(initial_state).unwrap();
			let post: PostRequest = post.try_into()?;

			// Obtaining the request stream and the timeout stream
			let timed_out =
				internals::message_timeout_stream(post.timeout_timestamp, self.source.clone())
					.await;

			let request_status = internals::post_request_status_stream(&self, post, state).await?;

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

	/// Return the status of a post request as a `ReadableStream` that yields
	/// `MessageStatusWithMeta`
	pub async fn get_request_status_stream(
		&self,
		request: JsValue,
		initial_state: JsValue,
	) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
		let lambda = || async move {
			let get = serde_wasm_bindgen::from_value::<JsGet>(request).unwrap();
			let get: GetRequest = get.try_into()?;
			let state =
				serde_wasm_bindgen::from_value::<MessageStatusStreamState>(initial_state).unwrap();

			// Obtaining the request stream and the timeout stream
			let timed_out =
				internals::message_timeout_stream(get.timeout_timestamp, self.hyperbridge.clone())
					.await;

			let request_status = internals::get_request_status_stream(&self, get, state).await?;
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
		request: JsValue,
		initial_state: JsValue,
	) -> Result<wasm_streams::readable::sys::ReadableStream, JsError> {
		let lambda = || async move {
			let post = serde_wasm_bindgen::from_value::<JsPost>(request).unwrap();
			let state =
				serde_wasm_bindgen::from_value::<TimeoutStreamState>(initial_state).unwrap();
			let post: PostRequest = post.try_into()?;

			let stream =
				internals::timeout_post_request_stream(&self, post, state).await?.map(|value| {
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

	#[cfg(feature = "tracing")]
	{
		use tracing_subscriber_wasm::MakeConsoleWriter;

		tracing_subscriber::fmt()
			.with_max_level(tracing::Level::TRACE)
			.with_writer(
				// To avoide trace events in the browser from showing their
				// JS backtrace, which is very annoying, in my opinion
				MakeConsoleWriter::default().map_trace_level_to(tracing::Level::INFO),
			)
			// For some reason, if we don't do this in the browser, we get
			// a runtime error.
			.without_time()
			.init();
	}

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
