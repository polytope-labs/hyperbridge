// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! RPC for `pallet-beefy-consensus-proofs`.
//!
//! Mirrors `ismp_queryEvents`: takes a block range and returns every
//! `ProofAccepted` event the pallet emitted in that range, keyed by block
//! hash (`String`, since JSON map keys must be strings). Replaces the
//! relayer's client-side "fetch each block's events one by one" loop with
//! a single server-side walk over the range.

#![deny(missing_docs)]

use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use pallet_beefy_consensus_proofs::types::ProofAcceptedEvent;
use pallet_beefy_consensus_proofs_runtime_api::BeefyConsensusProofsRuntimeApi;
use pallet_ismp_rpc::{runtime_error_into_rpc_error, BlockNumberOrHash};
use polkadot_sdk::*;
use sc_client_api::BlockBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, Header};
use std::{collections::HashMap, sync::Arc};

/// RPC surface for querying ranges of `ProofAccepted` events.
#[rpc(client, server)]
pub trait BeefyConsensusProofsApi<Hash> {
	/// Return every `ProofAccepted` event emitted between `from` and `to`
	/// (inclusive on both ends), keyed by block hash. Mirrors the shape of
	/// `ismp_queryEvents` so clients can reuse the same range/cursor loop.
	#[method(name = "ismp_queryProofAcceptedEvents")]
	fn query_proof_accepted_events(
		&self,
		from: BlockNumberOrHash<Hash>,
		to: BlockNumberOrHash<Hash>,
	) -> RpcResult<HashMap<String, Vec<ProofAcceptedEvent>>>;
}

/// Handler implementing [`BeefyConsensusProofsApi`].
pub struct BeefyConsensusProofsRpcHandler<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> BeefyConsensusProofsRpcHandler<C, B> {
	/// Construct a new handler.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block> BeefyConsensusProofsApiServer<Block::Hash>
	for BeefyConsensusProofsRpcHandler<C, Block>
where
	Block: BlockT,
	C: Send
		+ Sync
		+ 'static
		+ ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ BlockBackend<Block>,
	C::Api: BeefyConsensusProofsRuntimeApi<Block>,
{
	fn query_proof_accepted_events(
		&self,
		from: BlockNumberOrHash<Block::Hash>,
		to: BlockNumberOrHash<Block::Hash>,
	) -> RpcResult<HashMap<String, Vec<ProofAcceptedEvent>>> {
		let mut events = HashMap::new();

		let to =
			match to {
				BlockNumberOrHash::Hash(h) => h,
				BlockNumberOrHash::Number(n) =>
					self.client.block_hash(n.into()).ok().flatten().ok_or_else(|| {
						runtime_error_into_rpc_error("Invalid block number provided")
					})?,
			};
		let from =
			match from {
				BlockNumberOrHash::Hash(h) => h,
				BlockNumberOrHash::Number(n) =>
					self.client.block_hash(n.into()).ok().flatten().ok_or_else(|| {
						runtime_error_into_rpc_error("Invalid block number provided")
					})?,
			};

		let from_block = self
			.client
			.header(from)
			.map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
			.ok_or_else(|| runtime_error_into_rpc_error("Invalid block number or hash provided"))?;

		let mut header = self
			.client
			.header(to)
			.map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
			.ok_or_else(|| runtime_error_into_rpc_error("Invalid block number or hash provided"))?;

		// Walk parent pointers from `to` down to `from` so a single fork is
		// traversed, matching how `ismp_queryEvents` handles the same range.
		while header.number() >= from_block.number() {
			let api = self.client.runtime_api();
			let at = header.hash();

			let block_events: Vec<ProofAcceptedEvent> =
				api.proof_accepted_events(at).map_err(|e| {
					runtime_error_into_rpc_error(format!(
						"failed to read proof_accepted events {e:?}"
					))
				})?;

			if !block_events.is_empty() {
				events.insert(format!("{:?}", header.hash()), block_events);
			}

			if header.number() == from_block.number() {
				break;
			}

			header = self
				.client
				.header(*header.parent_hash())
				.map_err(|e| runtime_error_into_rpc_error(e.to_string()))?
				.ok_or_else(|| {
					runtime_error_into_rpc_error("Invalid block number or hash provided")
				})?;
		}

		Ok(events)
	}
}
