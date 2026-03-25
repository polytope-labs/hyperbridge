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

use anyhow::anyhow;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use subxt::{
	config::{ExtrinsicParams, HashFor},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};

pub use beefy_verifier_primitives::ConsensusState;
use host::{BeefyHost, BeefyHostConfig};
use ismp::host::StateMachine;
use prover::{Prover, ProverConfig};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

pub mod backend;
pub mod host;
pub mod prover;

const VALIDATOR_SET_ID_KEY: [u8; 32] =
	hex_literal::hex!("08c41974a97dbf15cfbec28365bea2da8f05bccc2f70ec66a32999c5761156be");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyConfig {
	// Configuration options for the BEEFY prover
	#[serde(flatten)]
	pub prover: ProverConfig,
	/// Configuration options for the beefy host
	#[serde(flatten)]
	pub host: BeefyHostConfig,
	/// Configuration options for the prover (state machines and finalization)
	#[serde(flatten)]
	pub prover_config: prover::BeefyProverConfig,
	/// substrate config
	#[serde(flatten)]
	pub substrate: SubstrateConfig,
}

impl BeefyConfig {
	/// Constructs an instance of the [`IsmpHost`] from the provided configs using Redis backend
	pub async fn into_client<R, P>(
		self,
	) -> Result<BeefyHost<R, P, zk_beefy::LocalProver, backend::RedisProofBackend>, anyhow::Error>
	where
		R: subxt::Config + Send + Sync + Clone,
		P: subxt::Config + Send + Sync + Clone,
		<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
		P::Signature: From<MultiSignature> + Send + Sync,
		P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
		H256: From<HashFor<P>>,
	{
		let client = SubstrateClient::<P>::new(self.substrate).await?;
		let prover = Prover::<R, P, zk_beefy::LocalProver>::new(self.prover.clone()).await?;

		// Create Redis-based unified backend (required for into_client)
		let redis_config =
			self.host.redis.as_ref().ok_or_else(|| {
				anyhow::anyhow!("Redis configuration is required for into_client")
			})?;
		let mut config = redis_config.clone();
		config.realtime = true; // Enable real-time notifications
		let backend = Arc::new(backend::RedisProofBackend::new(config).await?);

		let host =
			BeefyHost::<R, P, zk_beefy::LocalProver, _>::new(self.host, prover, client, backend)
				.await?;

		Ok(host)
	}
}

pub(crate) fn extract_para_id(state_machine: StateMachine) -> Result<u32, anyhow::Error> {
	let para_id = match state_machine {
		StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
		_ => Err(anyhow!("Invalid state machine: {state_machine}"))?,
	};

	Ok(para_id)
}
