// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Logic for canonicalizing MMR offchain entries for finalized forks,
//! and for pruning MMR offchain entries for stale forks.

#![warn(missing_docs)]

use crate::{aux_schema, HashFor, MmrClient, LOG_TARGET};
use codec::Decode;
use log::{debug, error, info, trace, warn};
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::{FullLeaf, Leaf};
use pallet_mmr_runtime_api::MmrRuntimeApi;
use polkadot_sdk::*;
use sc_client_api::{Backend, FinalityNotification};
use sc_offchain::OffchainDb;
use sp_blockchain::{CachedHeaderMetadata, ForkBackend};
use sp_core::{
	keccak_256,
	offchain::{DbExternalities, StorageKind},
};
use sp_mmr_primitives::{utils::NodesUtils, LeafIndex, NodeIndex};
use sp_runtime::{
	traits::{Block, Header, Keccak256, NumberFor, One},
	Saturating,
};
use std::{collections::VecDeque, sync::Arc};

/// `OffchainMMR` exposes MMR offchain canonicalization and pruning logic.
pub struct OffchainMmr<B: Block, BE: Backend<B>, C> {
	backend: Arc<BE>,
	client: Arc<C>,
	offchain_db: OffchainDb<BE::OffchainStorage>,
	indexing_prefix: Vec<u8>,
	first_mmr_block: NumberFor<B>,
	best_canonicalized: NumberFor<B>,
}

impl<B, BE, C> OffchainMmr<B, BE, C>
where
	BE: Backend<B>,
	B: Block,
	C: MmrClient<B, BE>,
	C::Api: MmrRuntimeApi<B, HashFor<B>, NumberFor<B>, Leaf>,
{
	pub fn new(
		backend: Arc<BE>,
		client: Arc<C>,
		offchain_db: OffchainDb<BE::OffchainStorage>,
		indexing_prefix: Vec<u8>,
		first_mmr_block: NumberFor<B>,
	) -> Option<Self> {
		let mut best_canonicalized = first_mmr_block.saturating_sub(One::one());
		best_canonicalized = aux_schema::load_or_init_state::<B, BE>(&*backend, best_canonicalized)
			.map_err(|e| error!(target: LOG_TARGET, "Error loading state from aux db: {:?}", e))
			.ok()?;

		Some(Self {
			backend,
			client,
			offchain_db,
			indexing_prefix,
			first_mmr_block,
			best_canonicalized,
		})
	}

	fn node_temp_offchain_key(&self, pos: NodeIndex, fork_identifier: B::Hash) -> Vec<u8> {
		NodesUtils::node_temp_offchain_key::<B::Header>(&self.indexing_prefix, pos, fork_identifier)
	}

	fn node_canon_offchain_key(&self, pos: NodeIndex) -> Vec<u8> {
		NodesUtils::node_canon_offchain_key(&self.indexing_prefix, pos)
	}

	fn write_gadget_state_or_log(&self) {
		if let Err(e) =
			aux_schema::write_gadget_state::<B, BE>(&*self.backend, &self.best_canonicalized)
		{
			debug!(target: LOG_TARGET, "error saving state: {:?}", e);
		}
	}

	fn header_metadata_or_log(
		&self,
		hash: B::Hash,
		action: &str,
	) -> Option<CachedHeaderMetadata<B>> {
		match self.client.header_metadata(hash) {
			Ok(header) => Some(header),
			_ => {
				debug!(
					target: LOG_TARGET,
					"Block {} not found. Couldn't {} associated branch.", hash, action
				);
				None
			},
		}
	}

	/// Fetch all the positions for all nodes added between these leaf counts including the leaf
	/// positions
	fn nodes_added_by_new_leaves(
		&self,
		block_num: NumberFor<B>,
		action: &str,
		old_leaf_count: LeafIndex,
		new_leaf_count: LeafIndex,
	) -> Vec<NodeIndex> {
		let mut nodes = vec![];
		for leaf_index in old_leaf_count..new_leaf_count {
			let branch = NodesUtils::right_branch_ending_in_leaf(leaf_index);
			nodes.extend(&branch);
		}
		debug!(target: LOG_TARGET, "Nodes to {} for block {}: {:?}", action, block_num, nodes);
		nodes
	}

	fn prune_branch(&mut self, block_hash: &B::Hash) {
		let action = "prune";
		let header = match self.header_metadata_or_log(*block_hash, action) {
			Some(header) => header,
			_ => return,
		};

		let parent_hash = header.parent;
		let parent_leaf_count = match self.client.runtime_api().mmr_leaf_count(parent_hash) {
			Ok(Ok(leaf_count)) => leaf_count,
			_ => {
				debug!(target: LOG_TARGET, "Failed to fetch mmr leaf count for {:?}", parent_hash);
				return;
			},
		};

		let current_leaf_count = match self.client.runtime_api().mmr_leaf_count(header.hash) {
			Ok(Ok(leaf_count)) => leaf_count,
			_ => {
				debug!(target: LOG_TARGET, "Failed to fetch mmr leaf count for {:?}", header.hash);
				return;
			},
		};

		let fork_identifier = match self.client.runtime_api().fork_identifier(header.hash) {
			Ok(Ok(fork_identifier)) => fork_identifier,
			_ => {
				debug!(target: LOG_TARGET, "Failed to fetch  fork_identifier at {:?}", header.hash);
				return;
			},
		};

		// We prune the leaf associated with the provided block and all the nodes added by that
		// leaf.
		let stale_nodes = self.nodes_added_by_new_leaves(
			header.number,
			action,
			parent_leaf_count,
			current_leaf_count,
		);

		for pos in stale_nodes {
			// Only prune nodes that have been moved to the canonical path to prevent deleting nodes
			// from forks that share the same child trie root before they are finalized.
			let canon_key = self.node_canon_offchain_key(pos);
			if let Some(_) = self.offchain_db.local_storage_get(StorageKind::PERSISTENT, &canon_key)
			{
				let temp_key = self.node_temp_offchain_key(pos, fork_identifier);
				self.offchain_db.local_storage_clear(StorageKind::PERSISTENT, &temp_key);
				debug!(target: LOG_TARGET, "Pruned elem at pos {} fork_identifier {:?} header_hash {:?}", pos, fork_identifier, block_hash);
			}
		}
	}

	fn canonicalize_branch(&mut self, block_hash: B::Hash) {
		let action = "canonicalize";
		let header = match self.header_metadata_or_log(block_hash, action) {
			Some(header) => header,
			_ => return,
		};

		// Don't canonicalize branches corresponding to blocks for which the MMR pallet
		// wasn't yet initialized.
		// Or headers less than the best canonicalized
		if header.number < self.first_mmr_block || header.number <= self.best_canonicalized {
			return;
		}

		let parent_hash = header.parent;
		let parent_leaf_count = match self.client.runtime_api().mmr_leaf_count(parent_hash) {
			Ok(Ok(leaf_count)) => leaf_count,
			_ => {
				debug!(target: LOG_TARGET, "Failed to fetch mmr leaf count for {:?}", parent_hash);
				return;
			},
		};

		let current_leaf_count = match self.client.runtime_api().mmr_leaf_count(header.hash) {
			Ok(Ok(leaf_count)) => leaf_count,
			_ => {
				debug!(target: LOG_TARGET, "Failed to fetch mmr leaf count for {:?}", header.hash);
				return;
			},
		};

		let fork_identifier = match self.client.runtime_api().fork_identifier(header.hash) {
			Ok(Ok(fork_identifier)) => fork_identifier,
			_ => {
				debug!(target: LOG_TARGET, "Failed to fetch  fork_identifier at {:?}", header.hash);
				return;
			},
		};

		// We "canonicalize" the leaves associated with the provided block
		// and all the nodes added by those leaves.
		let to_canon_nodes = self.nodes_added_by_new_leaves(
			header.number,
			action,
			parent_leaf_count,
			current_leaf_count,
		);

		for pos in to_canon_nodes {
			let temp_key = self.node_temp_offchain_key(pos, fork_identifier);
			if let Some(elem) =
				self.offchain_db.local_storage_get(StorageKind::PERSISTENT, &temp_key)
			{
				let canon_key = self.node_canon_offchain_key(pos);
				self.offchain_db.local_storage_set(StorageKind::PERSISTENT, &canon_key, &elem);
				self.offchain_db.local_storage_clear(StorageKind::PERSISTENT, &temp_key);
				// if it's a leaf node, also clear the duplicate that was added to no_op storage
				if let Ok(DataOrHash::Data(leaf)) =
					pallet_mmr_tree::mmr::Node::<Keccak256, Leaf>::decode(&mut &*elem)
				{
					let pre_image = leaf.preimage();
					let commitment = keccak_256(&pre_image);
					let duplicate_key = pallet_ismp::offchain::leaf_default_key(commitment.into());
					self.offchain_db.local_storage_clear(StorageKind::PERSISTENT, &duplicate_key);
				}
				debug!(
					target: LOG_TARGET,
					"Moved elem at pos {}, fork_identifier {:?} header_hash {:?} to canon key {:?}",
					pos,
					fork_identifier,
					block_hash,
					canon_key
				);
			} else {
				debug!(
					target: LOG_TARGET,
					"Couldn't canonicalize elem at pos {}, fork_identifier {:?} header_hash {:?}", pos, fork_identifier, block_hash
				);
			}
		}
		if self.best_canonicalized != header.number.saturating_sub(One::one()) {
			warn!(
				target: LOG_TARGET,
				"Detected canonicalization skip: best {:?} current {:?}.",
				self.best_canonicalized,
				header.number,
			);
			// If a block has been skipped canonicalize all blocks in between
			self.canonicalize_catch_up(header.parent);
		}
		self.best_canonicalized = header.number;
	}

	/// In case of missed finality notifications (node restarts for example),
	/// make sure to also canon everything leading up to `notification.tree_route`.
	pub fn canonicalize_catch_up(&mut self, first: HashFor<B>) {
		if let Some(mut header) = self.header_metadata_or_log(first, "canonicalize") {
			let mut to_canon = VecDeque::<<B as Block>::Hash>::new();
			// Walk up the chain adding all blocks newer than `self.best_canonicalized`.
			loop {
				header = match self.header_metadata_or_log(header.parent, "canonicalize") {
					Some(header) => header,
					_ => break,
				};
				if header.number <= self.best_canonicalized {
					break;
				}
				to_canon.push_front(header.hash);
			}
			// Canonicalize all blocks leading up to current finality notification.
			for hash in to_canon.drain(..) {
				self.canonicalize_branch(hash);
			}
			self.write_gadget_state_or_log();
		}
	}

	fn handle_potential_pallet_reset(&mut self, notification: &FinalityNotification<B>) {
		if let Some(first_mmr_block_num) = self.client.first_mmr_block_num(&notification) {
			if first_mmr_block_num != self.first_mmr_block {
				info!(
					target: LOG_TARGET,
					"pallet-mmr reset detected at block {:?} with new genesis at block {:?}",
					notification.header.number(),
					first_mmr_block_num
				);
				self.first_mmr_block = first_mmr_block_num;
				self.best_canonicalized = first_mmr_block_num.saturating_sub(One::one());
				self.write_gadget_state_or_log();
			}
		}
	}

	/// Move leafs and nodes added by finalized blocks in offchain db from _fork-aware key_ to
	/// _canonical key_.
	/// Prune leafs and nodes added by stale blocks in offchain db from _fork-aware key_.
	pub fn canonicalize_and_prune(&mut self, notification: FinalityNotification<B>) {
		// Update the first MMR block in case of a pallet reset.
		self.handle_potential_pallet_reset(&notification);

		// Run catchup to just to be sure no blocks are skipped
		self.canonicalize_catch_up(notification.hash);

		trace!(target: LOG_TARGET, "Got new finality notification {:?}, Best Canonicalized {:?}", notification.hash, self.best_canonicalized);
		let mut block_nums = vec![];
		for hash in notification.tree_route.iter().chain(std::iter::once(&notification.hash)) {
			match self.header_metadata_or_log(*hash, "canonicalize") {
				Some(header) => {
					block_nums.push(header.number);
				},
				_ => {},
			};
		}

		trace!(target: LOG_TARGET, "Canonicalizing Blocks{block_nums:?}");
		// Move offchain MMR nodes for finalized blocks to canonical keys.
		for hash in notification.tree_route.iter().chain(std::iter::once(&notification.hash)) {
			self.canonicalize_branch(*hash);
		}
		self.write_gadget_state_or_log();

		// Remove offchain MMR nodes for stale forks.
		let stale_forks = match self.client.expand_forks(&notification.stale_heads) {
			Ok(forks) => forks,
			Err(e) => {
				warn!(target: LOG_TARGET, "{:?}", e);
				return;
			},
		};
		for hash in stale_forks.iter() {
			self.prune_branch(hash);
		}
	}
}
