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
use host::{BeefyHost, BeefyHostConfig};
use ismp::host::StateMachine;
use prover::{Prover, ProverConfig};
use serde::{Deserialize, Serialize};

use subxt::{
	config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
	ext::sp_runtime::MultiSignature,
};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
pub use zk_beefy::Network;

mod byzantine;
pub mod host;
pub mod prover;
mod rsmq;

const VALIDATOR_SET_ID_KEY: [u8; 32] =
	hex_literal::hex!("08c41974a97dbf15cfbec28365bea2da8f05bccc2f70ec66a32999c5761156be");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyConfig {
	// Configuration options for the BEEFY prover
	pub prover: ProverConfig,
	/// Configuration options for the beefy prover and host
	pub host: BeefyHostConfig,
	/// substrate config
	pub substrate: SubstrateConfig,
}

impl BeefyConfig {
	/// Constructs an instance of the [`IsmpHost`] from the provided configs
	pub async fn into_client<R, P>(self) -> Result<BeefyHost<R, P>, anyhow::Error>
	where
		R: subxt::Config + Send + Sync + Clone,
		P: subxt::Config + Send + Sync + Clone,
		<P::ExtrinsicParams as ExtrinsicParams<P::Hash>>::OtherParams:
			Default + Send + Sync + From<BaseExtrinsicParamsBuilder<P, PlainTip>>,
		P::Signature: From<MultiSignature> + Send + Sync,
		P::AccountId:
			From<sp_core::crypto::AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	{
		let client = SubstrateClient::<P>::new(self.substrate).await?;
		let prover = Prover::<R, P>::new(self.prover.clone()).await?;
		let host = BeefyHost::<R, P>::new(self.host, prover, client).await?;

		Ok(host)
	}
}

pub(crate) fn extract_para_id(state_machine: StateMachine) -> Result<u32, anyhow::Error> {
	let para_id = match state_machine {
		StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
		_ => Err(anyhow!("Invalid state machine: {state_machine:?}"))?,
	};

	Ok(para_id)
}
