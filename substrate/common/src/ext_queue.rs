// Copyright (C) 2023 Polytope Labs.
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

//! Extrinsic queue for pipelining extrinsic submission to the parachain.

use primitives::queue::PipelineQueue;
use sp_core::sr25519;
use subxt::{
	config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
	ext::sp_runtime::MultiSignature,
	OnlineClient,
};

use crate::extrinsic::{send_extrinsic, Extrinsic, InMemorySigner};

/// Use this to initialize the extrinsic submit queue. This pipelines extrinsic submission
/// eliminating race conditions.
pub fn init_queue<T: subxt::Config>(
	client: OnlineClient<T>,
	pair: sr25519::Pair,
) -> anyhow::Result<PipelineQueue<Extrinsic>>
where
	T: Send + Sync + Clone,
	<T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
	T::Signature: From<MultiSignature> + Send + Sync,
	T::AccountId:
		From<sp_core::crypto::AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
{
	let signer = InMemorySigner::new(pair);

	let queue = primitives::queue::start_pipeline(move |extrinsic| {
		let client = client.clone();
		let signer = signer.clone();
		async move {
			let progress = match send_extrinsic(&client, signer, extrinsic).await {
				Ok(p) => p,
				Err(err) => {
					log::error!("Error sending extrinsic: {err:?}");
					return
				},
			};

			let extrinsic = match progress.wait_for_in_block().await {
				Ok(p) => p,
				Err(err) => {
					log::error!("Error waiting for extrinsic in_block {err:?}");
					return
				},
			};

			match extrinsic.wait_for_success().await {
				Ok(p) => p,
				Err(err) => {
					log::error!("Error executing extrinsic: {err:?}");
					return
				},
			};
		}
	});

	Ok(queue)
}
