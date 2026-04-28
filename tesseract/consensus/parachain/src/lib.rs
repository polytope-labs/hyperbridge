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
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tesseract_substrate_evm::{SubstrateEvmClient, SubstrateEvmClientConfig};

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
	/// Convert the config into an `IsmpHost` client backed by a
	/// [`SubstrateClient`]. Caller supplies the self chain's
	/// [`SubstrateConfig`]; that's paired with this config the same way
	/// `GrandpaConfig::into_client` is — the two live alongside each other
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
		let host = ParachainHost::<R>::from_substrate::<S>(&substrate, self).await?;
		Ok(Arc::new(host))
	}

	/// Convert the config into an `IsmpHost` client backed by a
	/// [`SubstrateEvmClient`]. Use this on parachains that expose an EVM
	/// surface (e.g. pallet-revive) — the consensus client side is the
	/// same as a plain substrate parachain, but the IsmpProvider has to
	/// reach EVM contracts on self.
	pub async fn into_substrate_evm_client<S, R>(
		&self,
		substrate_evm: SubstrateEvmClientConfig,
	) -> anyhow::Result<Arc<dyn IsmpHost>>
	where
		S: subxt::Config + Send + Sync + Clone + 'static,
		S::Header: Send + Sync,
		S::AccountId: From<AccountId32>
			+ Into<S::Address>
			+ Clone
			+ 'static
			+ Send
			+ Sync
			+ codec::Encode,
		S::Signature: From<MultiSignature> + Send + Sync,
		<S::ExtrinsicParams as ExtrinsicParams<S>>::Params: Send + Sync + DefaultParams,
		H256: From<HashFor<S>>,
		R: subxt::Config + Send + Sync + Clone + 'static,
		R::Header: Send + Sync,
		HashFor<R>: From<H256>,
	{
		let host = ParachainHost::<R>::from_substrate_evm::<S>(substrate_evm, self).await?;
		Ok(Arc::new(host))
	}
}

/// A parachain-consensus relayer host.
///
/// `R` is the subxt `Config` for the relay chain (typically `BlakeTwo256`
/// hasher). The self-chain client is held behind `Arc<dyn IsmpProvider>`
/// — both [`SubstrateClient`] (plain substrate parachain) and
/// [`SubstrateEvmClient`] (substrate parachain with an EVM surface, e.g.
/// pallet-revive) work, picked at construction via [`Self::from_substrate`]
/// vs [`Self::from_substrate_evm`].
#[derive(Clone)]
pub struct ParachainHost<R: subxt::Config> {
	/// Id of the consensus state on the counterparty — `DOT0` or `PAS0`
	/// depending on the relay chain. Optionally overridden via
	/// `SubstrateConfig::consensus_state_id`.
	pub consensus_state_id: ConsensusStateId,
	/// Self's state machine — the `StateMachine::Kusama(para_id)` /
	/// `StateMachine::Polkadot(para_id)` variant.
	pub state_machine: StateMachine,
	/// IsmpProvider for self (the parachain being relayed). Concrete type
	/// is either [`SubstrateClient`] or [`SubstrateEvmClient`] depending on
	/// which constructor was used.
	pub provider: Arc<dyn IsmpProvider>,
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

impl<R> ParachainHost<R>
where
	R: subxt::Config + Send + Sync + Clone,
{
	/// Construct from a pre-built [`IsmpProvider`].
	///
	/// `state_machine` is read off `provider.state_machine_id()` and must
	/// be a Polkadot/Kusama parachain variant. `consensus_state_id` is
	/// supplied by the caller — either a 4-byte override from the relayer
	/// config or the relay-derived default produced by
	/// [`parachain_consensus_state_id`].
	pub async fn new(
		provider: Arc<dyn IsmpProvider>,
		consensus_state_id: ConsensusStateId,
		config: &ParachainConfig,
	) -> Result<Self, anyhow::Error> {
		let state_machine = provider.state_machine_id().state_id;
		if !matches!(state_machine, StateMachine::Polkadot(_) | StateMachine::Kusama(_)) {
			return Err(anyhow!(
				"ParachainHost only supports Polkadot/Kusama parachains, got {state_machine}"
			));
		}

		// 150 MiB payload cap — relay state proofs can be chunky.
		let (relay_client, relay_rpc_client) =
			subxt_utils::client::ws_client::<R>(&config.relay_rpc_ws, 150 * 1024 * 1024).await?;
		let relay_rpc = LegacyRpcMethods::<R>::new(relay_rpc_client.clone());

		Ok(Self {
			consensus_state_id,
			state_machine,
			provider,
			relay_client,
			relay_rpc,
			relay_rpc_client,
			config: config.clone(),
		})
	}

	/// Convenience constructor that builds a [`SubstrateClient<S>`] from
	/// the supplied [`SubstrateConfig`] and uses it as the IsmpProvider.
	/// Use this for a plain substrate parachain.
	pub async fn from_substrate<S>(
		substrate: &SubstrateConfig,
		config: &ParachainConfig,
	) -> Result<Self, anyhow::Error>
	where
		S: subxt::Config + Send + Sync + Clone + 'static,
		S::Header: Send + Sync,
		S::Signature: From<MultiSignature> + Send + Sync,
		S::AccountId: From<AccountId32> + Into<S::Address> + Clone + 'static + Send + Sync,
		<S::ExtrinsicParams as ExtrinsicParams<S>>::Params: Send + Sync + DefaultParams,
		H256: From<HashFor<S>>,
	{
		let substrate_client = SubstrateClient::<S>::new(substrate.clone()).await?;
		let state_machine = substrate_client.state_machine();
		let consensus_state_id =
			resolve_consensus_state_id(substrate.consensus_state_id.as_deref(), state_machine)?;
		Self::new(Arc::new(substrate_client), consensus_state_id, config).await
	}

	/// Convenience constructor that builds a [`SubstrateEvmClient<S>`]
	/// from the supplied [`SubstrateEvmClientConfig`] and uses it as the
	/// IsmpProvider. Use this on parachains that expose an EVM surface
	/// (e.g. pallet-revive) — proofs/state queries hit the EVM contracts
	/// while consensus is still parachain-style.
	pub async fn from_substrate_evm<S>(
		substrate_evm: SubstrateEvmClientConfig,
		config: &ParachainConfig,
	) -> Result<Self, anyhow::Error>
	where
		S: subxt::Config + Send + Sync + Clone + 'static,
		S::Header: Send + Sync,
		S::Signature: From<MultiSignature> + Send + Sync,
		S::AccountId: From<AccountId32>
			+ Into<S::Address>
			+ Clone
			+ 'static
			+ Send
			+ Sync
			+ codec::Encode,
		<S::ExtrinsicParams as ExtrinsicParams<S>>::Params: Send + Sync + DefaultParams,
		H256: From<HashFor<S>>,
	{
		// `consensus_state_id` lives on the EVM half of the config — same
		// 4-byte override convention as the substrate variant.
		let consensus_override = substrate_evm.evm.consensus_state_id.clone();
		let client = SubstrateEvmClient::<S>::new(substrate_evm).await?;
		let state_machine = client.evm.state_machine;
		let consensus_state_id =
			resolve_consensus_state_id(consensus_override.as_deref(), state_machine)?;
		Self::new(Arc::new(client), consensus_state_id, config).await
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

/// Resolve the on-counterparty `ConsensusStateId` for a parachain. If the
/// caller supplied a 4-byte override (e.g. `DOT0`/`PAS0`/something custom)
/// use it; otherwise fall back to [`parachain_consensus_state_id`] which
/// derives the canonical id from the parachain's state machine.
fn resolve_consensus_state_id(
	override_str: Option<&str>,
	state_machine: StateMachine,
) -> Result<ConsensusStateId, anyhow::Error> {
	match override_str {
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
			Ok(id)
		},
		None => Ok(parachain_consensus_state_id(state_machine)),
	}
}
