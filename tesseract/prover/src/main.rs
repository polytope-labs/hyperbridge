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

//! Standalone BEEFY consensus prover daemon.

use anyhow::anyhow;
use clap::Parser;
use primitive_types::H256;
use std::sync::Arc;
use tesseract_beefy::prover::{BeefyProver, Prover};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient,
};
use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

/// CLI interface for BEEFY prover
#[derive(Parser, Debug)]
#[command(name = "tesseract-prover", about = "Standalone BEEFY consensus prover daemon")]
pub struct Cli {
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	tracing_subscriber::fmt().with_env_filter(filter).finish().try_init()?;

	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");
	let cli = Cli::parse();

	let mut config = tokio::fs::read_to_string(cli.config).await?.parse::<toml::Table>()?;

	let substrate = {
		let substrate_config: tesseract_substrate::SubstrateConfig = config
			.remove("substrate")
			.ok_or_else(|| anyhow!("Substrate config missing; qed"))?
			.try_into()?;
		// Auto-fill state_machine + consensus_state_id from the chain when omitted.
		SubstrateClient::new(substrate_config.resolve().await?).await?
	};

	// The SP1 nonce committed into each proof must equal the account that submits it, which is
	// this prover's substrate signer (see `pallet-beefy-consensus-proofs`). `address` is the
	// signer's 32-byte sr25519 public key.
	let account: H256 = <[u8; 32]>::try_from(substrate.address.as_slice())
		.map_err(|_| anyhow!("beefy submission signer account must be 32 bytes"))?
		.into();

	let prover = {
		let prover_config = config
			.remove("prover")
			.ok_or_else(|| anyhow!("Substrate config missing; qed"))?;
		Prover::new(prover_config.try_into()?, account).await?
	};

	let beefy_prover = {
		let beefy_config: tesseract_beefy::prover::BeefyProverConfig = config
			.remove("beefy")
			.ok_or_else(|| anyhow!("Substrate config missing; qed"))?
			.try_into()?;

		let backend: Arc<dyn tesseract_beefy::backend::ProofBackend> = match beefy_config.backend {
			tesseract_beefy::backend::ProofBackendConfig::Redis { ref config } => {
				let mut cfg = config.clone();
				cfg.realtime = true;
				Arc::new(tesseract_beefy::backend::RedisProofBackend::new(cfg).await?)
			},
			tesseract_beefy::backend::ProofBackendConfig::Onchain => {
				let mut state_machine_id = substrate.state_machine_id();
				state_machine_id.consensus_state_id = beefy_config.consensus_state_id;
				Arc::new(tesseract_beefy::backend::OnchainBackend::<KeccakSubstrateChain>::new(
					substrate.client.clone(),
					substrate.rpc_client.clone(),
					substrate.signer.clone(),
					state_machine_id,
				))
			},
			ref b => Err(anyhow!("Unsupported backend configuration: {b:?}"))?,
		};

		BeefyProver::<
			Blake2SubstrateChain,
			KeccakSubstrateChain,
			zk_beefy::LocalProver,
			dyn tesseract_beefy::backend::ProofBackend,
		>::new(beefy_config, substrate, prover, backend)
		.await?
	};

	// run the prover (queues are initialized in new())
	beefy_prover.run().await;

	Ok(())
}
