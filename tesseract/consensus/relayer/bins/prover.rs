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
//

//! BEEFY consensus prover

use anyhow::anyhow;
use clap::{arg, Parser};
use tesseract_beefy::prover::{BeefyProver, Prover};
use tesseract_consensus::logging;
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient,
};

/// CLI interface for BEEFY prover
#[derive(Parser, Debug)]
pub struct Cli {
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	logging::setup()?;
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");
	let cli = Cli::parse();

	let mut config = tokio::fs::read_to_string(cli.config).await?.parse::<toml::Table>()?;

	let prover = {
		let prover_config = config
			.remove("prover")
			.ok_or_else(|| anyhow!("Substrate config missing; qed"))?;
		Prover::new(prover_config.try_into()?).await?
	};

	let substrate = {
		let substrate_config = config
			.remove("substrate")
			.ok_or_else(|| anyhow!("Substrate config missing; qed"))?;
		SubstrateClient::new(substrate_config.try_into()?).await?
	};

	let mut beefy_prover = {
		let beefy_config =
			config.remove("beefy").ok_or_else(|| anyhow!("Substrate config missing; qed"))?;
		BeefyProver::<Blake2SubstrateChain, KeccakSubstrateChain>::new(
			beefy_config.try_into()?,
			substrate,
			prover,
		)
		.await?
	};

	beefy_prover.init_queues().await?;
	// run the prover
	beefy_prover.run().await;

	Ok(())
}
