// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Supermajority (2/3·N + 1) quorum primitives over a set of L2 RPC endpoints. Mirrors the
//! shape of [`tesseract_evm::byzantine`] but is parameterised over the value being polled so
//! both opstack (output_root) and arbitrum (block hash / state root) fishermen can reuse it.

use std::time::Duration;

use alloy::{eips::BlockId, primitives::B256, providers::Provider, rpc::types::Block};
use tesseract_evm::AlloyProvider;

/// Supermajority quorum threshold: `⌊2/3·N⌋ + 1`. Matches the BFT bound used elsewhere in
/// tesseract for byzantine detection.
pub fn quorum_threshold(total: usize) -> usize {
	total * 2 / 3 + 1
}

/// Outcome of polling one L2 provider, after retries.
pub enum FetchOutcome<T> {
	/// Provider returned the requested data.
	Found(T),
	/// Provider definitively reports the data isn't available (e.g. block not yet on this node).
	Missing,
	/// Provider failed with transport errors on every attempt. Non-signal.
	Errored,
}

/// Each per-provider fetch is retried up to this many times on transport errors before being
/// recorded as a non-signal. Transport errors do not by themselves justify a blacklist.
pub const MAX_TRANSPORT_RETRIES: usize = 3;

/// Backoff between retries.
pub const RETRY_BACKOFF: Duration = Duration::from_millis(250);

/// Fetch the block at `height` from a single provider, retrying transport errors before giving
/// up. `Ok(None)` is treated as a real "missing" signal — the block genuinely isn't on this
/// node yet — and returned immediately without further retries.
pub async fn fetch_block_by_number(provider: &AlloyProvider, height: u64) -> FetchOutcome<Block> {
	for attempt in 1..=MAX_TRANSPORT_RETRIES {
		match provider.get_block(BlockId::number(height)).await {
			Ok(Some(block)) => return FetchOutcome::Found(block),
			Ok(None) => return FetchOutcome::Missing,
			Err(e) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"quorum fetch-by-number attempt {attempt}/{MAX_TRANSPORT_RETRIES} for height {height} failed: {e:?}",
				);
				if attempt < MAX_TRANSPORT_RETRIES {
					tokio::time::sleep(RETRY_BACKOFF).await;
				}
			},
		}
	}
	FetchOutcome::Errored
}

/// Same as [`fetch_block_by_number`] but by block hash.
pub async fn fetch_block_by_hash(provider: &AlloyProvider, hash: B256) -> FetchOutcome<Block> {
	for attempt in 1..=MAX_TRANSPORT_RETRIES {
		match provider.get_block(BlockId::hash(hash)).await {
			Ok(Some(block)) => return FetchOutcome::Found(block),
			Ok(None) => return FetchOutcome::Missing,
			Err(e) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"quorum fetch-by-hash attempt {attempt}/{MAX_TRANSPORT_RETRIES} for {hash:?} failed: {e:?}",
				);
				if attempt < MAX_TRANSPORT_RETRIES {
					tokio::time::sleep(RETRY_BACKOFF).await;
				}
			},
		}
	}
	FetchOutcome::Errored
}

/// Decision derived from a fan-out poll of `N` providers against the claimed value.
pub enum QuorumDecision {
	/// At least the quorum threshold of providers agreed with the on-chain claim. No action.
	Verified,
	/// At least the quorum threshold of providers report the block / data is missing. The
	/// claim is then almost certainly fraudulent — blacklist.
	MissingFromQuorum,
	/// At least the quorum threshold of providers returned a value that disagrees with the
	/// claim. Blacklist.
	Mismatch,
	/// Below quorum on either side. Abstain — we don't have enough signal.
	InsufficientQuorum,
}

/// Aggregate a vec of [`FetchOutcome`] into a [`QuorumDecision`] given a predicate that says
/// whether a fetched value agrees with the on-chain claim.
pub fn decide<T>(outcomes: Vec<FetchOutcome<T>>, agrees: impl Fn(&T) -> bool) -> QuorumDecision {
	let quorum = quorum_threshold(outcomes.len());
	let (mut found, mut agree, mut missing) = (0usize, 0usize, 0usize);
	for outcome in outcomes {
		match outcome {
			FetchOutcome::Found(v) => {
				found += 1;
				if agrees(&v) {
					agree += 1;
				}
			},
			FetchOutcome::Missing => missing += 1,
			FetchOutcome::Errored => {},
		}
	}
	if missing >= quorum {
		return QuorumDecision::MissingFromQuorum;
	}
	if found < quorum {
		return QuorumDecision::InsufficientQuorum;
	}
	if agree >= quorum {
		QuorumDecision::Verified
	} else {
		QuorumDecision::Mismatch
	}
}
