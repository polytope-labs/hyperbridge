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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn quorum_threshold_is_supermajority() {
		assert_eq!(quorum_threshold(1), 1);
		assert_eq!(quorum_threshold(2), 2);
		assert_eq!(quorum_threshold(3), 3);
		assert_eq!(quorum_threshold(4), 3);
		assert_eq!(quorum_threshold(5), 4);
		assert_eq!(quorum_threshold(6), 5);
	}

	/// A single lagging RPC (one `Missing`) among a genuine N-way fan-out must NOT
	/// trigger a blacklist — it falls through to `InsufficientQuorum` (abstain).
	/// This is exactly the false-positive the quorum-of-one regression produced
	/// when all N URLs were collapsed into a single provider.
	#[test]
	fn single_lagging_rpc_among_quorum_abstains() {
		let outcomes =
			vec![FetchOutcome::Found(1u8), FetchOutcome::Found(1u8), FetchOutcome::Missing];
		assert!(matches!(
			decide(outcomes, |v| *v == 1),
			QuorumDecision::InsufficientQuorum
		));
	}

	/// But when a real quorum of providers reports the block missing we DO act —
	/// we don't abstain when the quorum genuinely returns the block as missing.
	#[test]
	fn quorum_of_missing_blacklists() {
		let outcomes: Vec<FetchOutcome<u8>> =
			vec![FetchOutcome::Missing, FetchOutcome::Missing, FetchOutcome::Missing];
		assert!(matches!(
			decide(outcomes, |_| true),
			QuorumDecision::MissingFromQuorum
		));
	}

	/// Below quorum on the missing side, abstain rather than blacklist: two of
	/// three missing is not enough when the threshold is three.
	#[test]
	fn sub_quorum_missing_abstains() {
		let outcomes =
			vec![FetchOutcome::Missing, FetchOutcome::Missing, FetchOutcome::Found(1u8)];
		assert!(matches!(
			decide(outcomes, |v| *v == 1),
			QuorumDecision::InsufficientQuorum
		));
	}

	/// Documents the hazard the fix removes: a *single* outcome makes
	/// `quorum_threshold(1) == 1`, so one `Missing` (or one disagreeing `Found`)
	/// is by itself a "quorum". Collapsing N RPC URLs into one provider is what
	/// produced this 1-element outcome vector in the rollup watcher.
	#[test]
	fn single_outcome_is_a_quorum_of_one() {
		let missing: Vec<FetchOutcome<u8>> = vec![FetchOutcome::Missing];
		assert!(matches!(
			decide(missing, |_| true),
			QuorumDecision::MissingFromQuorum
		));
		let wrong = vec![FetchOutcome::Found(2u8)];
		assert!(matches!(decide(wrong, |v| *v == 1), QuorumDecision::Mismatch));
	}

	#[test]
	fn unanimous_agreement_verifies() {
		let outcomes =
			vec![FetchOutcome::Found(1u8), FetchOutcome::Found(1u8), FetchOutcome::Found(1u8)];
		assert!(matches!(decide(outcomes, |v| *v == 1), QuorumDecision::Verified));
	}

	/// Transport errors are non-signals and never on their own drive a decision.
	#[test]
	fn all_errored_is_insufficient_quorum() {
		let outcomes: Vec<FetchOutcome<u8>> =
			vec![FetchOutcome::Errored, FetchOutcome::Errored, FetchOutcome::Errored];
		assert!(matches!(
			decide(outcomes, |_| true),
			QuorumDecision::InsufficientQuorum
		));
	}
}
