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

use polkadot_sdk::*;
use std::path::PathBuf;

/// Sub-commands supported by the collator.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Remove the whole chain.
	PurgeChain(cumulus_client_cli::PurgeChainCmd),

	/// Export the genesis state of the parachain.
	ExportGenesisState(cumulus_client_cli::ExportGenesisHeadCommand),

	/// Export the genesis wasm of the parachain.
	ExportGenesisWasm(cumulus_client_cli::ExportGenesisWasmCommand),

	/// Sub-commands concerned with benchmarking.
	/// The pallet benchmarking moved to the `pallet` sub-command.
	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Try some testing command against a specified runtime state.
	#[cfg(feature = "try-runtime")]
	TryRuntime(try_runtime_cli::TryRuntimeCmd),

	/// Errors since the binary was not build with `--features try-runtime`.
	#[cfg(not(feature = "try-runtime"))]
	TryRuntime,

	/// Runs the node with signature verification override and manual seal.
	Simnode(SimnodeCommand),
}

/// Wrapper around `sc_simnode::cli::SimnodeCli` that adds the hyperbridge
/// specific flags. Named distinctly from the inner type so clap's
/// argument group derivation does not collide on the shared `SimnodeCli`
/// group name.
#[derive(Debug, clap::Parser)]
pub struct SimnodeCommand {
	#[arg(long)]
	pub instant: bool,
	/// Path to a tesseract relayer toml config. Duplicated at the
	/// subcommand level because `args_conflicts_with_subcommands` on the
	/// root `Cli` makes top level options unreachable when running a
	/// subcommand. When simnode runs as a `--validator` the fisherman is
	/// spawned from this config, matching the production collator path.
	#[arg(long)]
	pub tesseract_config: Option<PathBuf>,
	#[command(flatten)]
	pub eth_rpc: EthRpcParams,
	#[command(flatten)]
	pub rest: sc_simnode::cli::SimnodeCli,
}

#[derive(Debug, Clone, clap::Args)]
pub struct EthRpcParams {
	#[arg(
		long,
		help = "Serve the Ethereum JSON-RPC facade (eth_getLogs, eth_getBlockByNumber, ...) for EVM indexers on the node RPC port; requires archive pruning"
	)]
	pub eth_rpc: bool,
	#[arg(
		long,
		default_value_t = 500,
		help = "Largest eth_getLogs span accepted (toBlock - fromBlock)"
	)]
	pub eth_rpc_max_block_range: u64,
	#[arg(long, default_value_t = 4096, help = "Decoded blocks kept in memory")]
	pub eth_rpc_cache_blocks: usize,
	#[arg(long, default_value_t = 8, help = "Parallel block decodes per eth_getLogs")]
	pub eth_rpc_concurrency: usize,
}

#[derive(Debug, Clone)]
pub struct EthRpcConfig {
	pub chain_name: String,
	pub eth_chain_id: u64,
	pub max_block_range: u64,
	pub cache_blocks: usize,
	pub concurrency: usize,
}

impl EthRpcParams {
	pub fn to_config(&self, chain_name: &str, para_id: u32) -> Option<EthRpcConfig> {
		if !self.eth_rpc {
			return None;
		}
		Some(EthRpcConfig {
			chain_name: chain_name.to_string(),
			eth_chain_id: 1_000_000_000 + para_id as u64,
			max_block_range: self.eth_rpc_max_block_range,
			cache_blocks: self.eth_rpc_cache_blocks,
			concurrency: self.eth_rpc_concurrency,
		})
	}
}

#[derive(Debug, clap::Parser)]
#[command(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[command(flatten)]
	pub run: cumulus_client_cli::RunCmd,

	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	/// Should we enable async-backing?
	///
	/// NOTE: DEPRECATED, async-backing is enabled by default.
	#[arg(long)]
	pub async_backing: bool,

	/// Path to a tesseract relayer toml config. When the node runs as a
	/// collator/authority this is required, and the fisherman task is spawned
	/// using it to watch L2 chains across multiple RPC providers and submit
	/// veto extrinsics on disagreement. When the node is a non-authority, the
	/// flag is optional and ignored.
	#[arg(long)]
	pub tesseract_config: Option<PathBuf>,

	#[command(flatten)]
	pub eth_rpc: EthRpcParams,

	/// Relay chain arguments
	#[arg(raw = true)]
	pub relay_chain_args: Vec<String>,
}

#[derive(Debug)]
pub struct RelayChainCli {
	/// The actual relay chain cli object.
	pub base: polkadot_cli::RunCmd,

	/// Optional chain id that should be passed to the relay chain.
	pub chain_id: Option<String>,

	/// The base path that should be used by the relay chain.
	pub base_path: Option<PathBuf>,
}

impl RelayChainCli {
	/// Parse the relay chain CLI parameters using the para chain `Configuration`.
	pub fn new<'a>(
		para_config: &sc_service::Configuration,
		relay_chain_args: impl Iterator<Item = &'a String>,
	) -> Self {
		let extension = crate::chain_spec::Extensions::try_get(&*para_config.chain_spec);
		let chain_id = extension.map(|e| e.relay_chain.clone());
		let base_path = para_config.base_path.path().join("polkadot");
		Self {
			base_path: Some(base_path),
			chain_id,
			base: clap::Parser::parse_from(relay_chain_args),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::EthRpcParams;

	fn params(eth_rpc: bool) -> EthRpcParams {
		EthRpcParams {
			eth_rpc,
			eth_rpc_max_block_range: 500,
			eth_rpc_cache_blocks: 4096,
			eth_rpc_concurrency: 8,
		}
	}

	#[test]
	fn eth_rpc_config_derives_chain_id_from_para_id() {
		let nexus = params(true).to_config("nexus", 3367).unwrap();
		assert_eq!(nexus.eth_chain_id, 1_000_003_367);
		assert_eq!(nexus.chain_name, "nexus");
		assert_eq!(nexus.max_block_range, 500);
		assert_eq!(nexus.cache_blocks, 4096);
		assert_eq!(nexus.concurrency, 8);

		let gargantua = params(true).to_config("gargantua-4009", 4009).unwrap();
		assert_eq!(gargantua.eth_chain_id, 1_000_004_009);
		assert_eq!(gargantua.chain_name, "gargantua-4009");
	}

	#[test]
	fn eth_rpc_config_is_none_when_flag_is_off() {
		assert!(params(false).to_config("nexus", 3367).is_none());
	}
}
