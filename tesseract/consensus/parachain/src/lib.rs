// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! # Parachain consensus client — relayer side
//!
//! Ships `ConsensusMessage`s of type [`ismp_parachain::consensus::ParachainConsensusProof`] from
//! one parachain (this host) to another (the counterparty — typically Hyperbridge).
//!
//! The counterparty's `pallet-ismp-parachain` maintains a bounded registry of recently-seen relay
//! chain state roots in [`KnownRelayHeights`]. A proof produced here references one of those relay
//! heights, carries a state-trie proof of `Paras::Heads[self_para_id]` against the matching relay
//! state root, and the counterparty's on-chain verifier extracts the encoded parachain header,
//! reads its `(timestamp, overlay_root, state_root)` out of the digest, and stores the resulting
//! state commitment.
//!
//! [`KnownRelayHeights`]: https://github.com/polytope-labs/hyperbridge/blob/main/modules/ismp/clients/parachain/client/src/lib.rs

pub const LOG_TARGET: &str = "consensus-parachain";

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{ExtrinsicParams, HashFor},
	ext::subxt_rpcs::RpcClient,
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
	OnlineClient,
};

use std::sync::Arc;

use ismp::{consensus::ConsensusStateId, host::StateMachine};
use ismp_parachain::parachain_consensus_state_id;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

mod host;

/// Poll cadence for the consensus loop.
pub const CONSENSUS_UPDATE_FREQUENCY: u64 = 30;

/// Config for the parachain consensus host. Pairs a host `SubstrateConfig` for
/// the parachain being relayed (self) with a relay-chain RPC URL. Everything
/// the consensus loop needs on self (signer, rpc, state_machine) comes from
/// the inner substrate config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParachainConfig {
	/// Relay chain WebSocket RPC URL.
	pub relay_rpc_ws: String,
}

impl ParachainConfig {
	/// Convert the config into an `IsmpHost` client. Caller supplies the self
	/// chain's [`SubstrateConfig`]; that's paired with this config the same
	/// way `GrandpaConfig::into_client` is — the two live alongside each other
	/// in the consolidated relayer's per-chain section.
	pub async fn into_client<S, R>(
		&self,
		substrate: SubstrateConfig,
	) -> anyhow::Result<Arc<dyn IsmpHost>>
	where
		S: subxt::Config + Send + Sync + Clone + 'static,
		S::Header: Send + Sync,
		S::AccountId: From<AccountId32> + Into<S::Address> + Clone + 'static + Send + Sync,
		S::Signature: From<MultiSignature> + Send + Sync,
		<S::ExtrinsicParams as ExtrinsicParams<S>>::Params: Send + Sync + DefaultParams,
		H256: From<HashFor<S>>,
		R: subxt::Config + Send + Sync + Clone + 'static,
		R::Header: Send + Sync,
		HashFor<R>: From<H256>,
	{
		let host = ParachainHost::<S, R>::new(&substrate, self).await?;
		Ok(Arc::new(host))
	}
}

/// A parachain-consensus relayer host.
///
/// - `S` is the subxt `Config` for self (the parachain we produce proofs for).
/// - `R` is the subxt `Config` for the relay chain (typically `BlakeTwo256` hasher).
#[derive(Clone)]
pub struct ParachainHost<S: subxt::Config, R: subxt::Config> {
	/// Id of the consensus state on the counterparty — `DOT0` or `PAS0`
	/// depending on the relay chain. Optionally overridden via
	/// `SubstrateConfig::consensus_state_id`.
	pub consensus_state_id: ConsensusStateId,
	/// Self's state machine — the `StateMachine::Kusama(para_id)` /
	/// `StateMachine::Polkadot(para_id)` variant.
	pub state_machine: StateMachine,
	/// Subxt client for self (the parachain being relayed).
	pub substrate_client: SubstrateClient<S>,
	/// Subxt online client for the relay chain.
	pub relay_client: OnlineClient<R>,
	/// Legacy RPC methods handle on the relay chain — used for `state_getReadProof`
	/// and header/block lookups.
	pub relay_rpc: LegacyRpcMethods<R>,
	/// Raw RPC client on the relay chain.
	pub relay_rpc_client: RpcClient,
	/// The parachain consensus config.
	pub config: ParachainConfig,
}

impl<S, R> ParachainHost<S, R>
where
	S: subxt::Config + Send + Sync + Clone,
	S::Signature: From<MultiSignature> + Send + Sync,
	S::AccountId: From<AccountId32> + Into<S::Address> + Clone + 'static + Send + Sync,
	<S::ExtrinsicParams as ExtrinsicParams<S>>::Params: Send + Sync + DefaultParams,
	H256: From<HashFor<S>>,
	R: subxt::Config + Send + Sync + Clone,
{
	pub async fn new(
		substrate: &SubstrateConfig,
		config: &ParachainConfig,
	) -> Result<Self, anyhow::Error> {
		let substrate_client = SubstrateClient::<S>::new(substrate.clone()).await?;
		let state_machine = substrate_client.state_machine();
		if !matches!(state_machine, StateMachine::Polkadot(_) | StateMachine::Kusama(_)) {
			return Err(anyhow!(
				"ParachainHost only supports Polkadot/Kusama parachains, got {state_machine}"
			));
		}

		// 150 MiB payload cap — relay state proofs can be chunky.
		let (relay_client, relay_rpc_client) =
			subxt_utils::client::ws_client::<R>(&config.relay_rpc_ws, 150 * 1024 * 1024).await?;
		let relay_rpc = LegacyRpcMethods::<R>::new(relay_rpc_client.clone());

		let consensus_state_id = match substrate.consensus_state_id.clone() {
			Some(raw) => {
				let bytes = raw.as_bytes();
				if bytes.len() != 4 {
					return Err(anyhow!(
						"consensus_state_id must be 4 bytes, got {} bytes ('{raw}')",
						bytes.len(),
					));
				}
				let mut id: ConsensusStateId = Default::default();
				id.copy_from_slice(bytes);
				id
			},
			None => parachain_consensus_state_id(state_machine),
		};

		Ok(Self {
			consensus_state_id,
			state_machine,
			substrate_client,
			relay_client,
			relay_rpc,
			relay_rpc_client,
			config: config.clone(),
		})
	}

	/// Para id of self, extracted from the configured state machine.
	pub fn para_id(&self) -> u32 {
		match self.state_machine {
			StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
			// checked in `new`
			_ => unreachable!("ParachainHost checked state machine at construction"),
		}
	}
}
