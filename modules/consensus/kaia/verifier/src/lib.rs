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

//! Verifier for Kaia's Istanbul BFT consensus.
//!
//! Kaia (mainnet chain id 8217, ex-Klaytn) finalizes every block instantly
//! through an Istanbul BFT (PBFT-family) protocol. Each header carries its own
//! finality artifact in `extra_data`: the proposer's secp256k1 seal plus the
//! committee's committed seals. This verifier mirrors the import-time check in
//! kaiachain/kaia `blockchain/block_validator.go`:
//!
//! - the recovered block author must be a council member,
//! - at least `2f + 1` distinct council members must have committed, where
//!   `f = ceil(min(|council|, committee_size) / 3) - 1`.
//!
//! The council is tracked incrementally from `governance.addvalidator` /
//! `governance.removevalidator` votes carried in header `vote` fields, and the
//! committee size / governing node from ratified governance at epoch blocks.
//!
//! # Council freshness is a trust assumption
//!
//! Updates may skip blocks — Kaia produces 86,400 a day, so following every
//! one is not viable — and nothing in a header proves the blocks before it
//! carried no votes. A submitter that advances the client past a vote-bearing
//! header therefore leaves the tracked council stale, and the skipped header
//! can never be applied afterwards because it now sits below the finalized
//! height. Whoever assembles updates is trusted to include every vote header
//! and every epoch block with governance data.
//!
//! Drift in the dangerous direction is caught rather than trusted: a skipped
//! `addvalidator` understates the council and so understates the quorum, and
//! [`verify_header_seals`] rejects any header declaring a validator the client
//! does not know ([`Error::CouncilDrift`]), turning silent divergence into a
//! visible stall. A skipped `removevalidator` cannot be detected from headers;
//! it leaves a departed validator counting toward quorum, which is why update
//! assembly must not be left to an untrusted party.
//!
//! # The quorum divisor
//!
//! The chain derives `f` from the number of *qualified* validators (council
//! members meeting the minimum stake), which depends on staking state and is
//! not derivable from headers. This verifier substitutes the council size,
//! which is never smaller. That is safe — it can only demand more seals than
//! the chain does — but it is not free: blocks carry about `ceil(2N/3)` seals
//! for `N = min(qualified, committee_size)`, so once enough council members
//! are demoted the requirement here exceeds what live blocks actually carry
//! and the client stalls until its state is refreshed by governance.
//!
//! # Permissionless hardfork
//!
//! These are Kaia's pre-Permissionless consensus rules, which is what mainnet
//! and Kairos run today (`PermissionlessCompatibleBlock` is unset on both).
//! The fork changes consensus in ways this client does not implement: committed
//! seals bind the round into their preimage, the quorum becomes `ceil(2n/3)`
//! counted over the round's *committee* rather than the council, the author
//! must equal the committee-selected proposer, and validator membership moves
//! from header votes to `AddressBookV2` contract state (KIP-290) — which this
//! client's header-derived council tracking cannot follow at all.
//!
//! If the fork activates before this client is upgraded, it fails safe rather
//! than accepting anything unsound: seal recovery over the old preimage yields
//! addresses that are not council members, so every header is rejected with
//! [`Error::InsufficientSeals`] and the client stalls until upgraded. Kaia
//! hardfork scheduling must therefore be monitored, and a redesign around
//! `AddressBookV2` state proofs shipped before activation.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use polkadot_sdk::*;

use alloc::collections::BTreeSet;
use ismp::messaging::Keccak256;
use primitive_types::{H160, H256, U256};

pub mod error;
pub mod primitives;

#[cfg(test)]
mod fixtures;

pub use error::Error;
use primitives::{
	committed_seal_message, header_hash, parse_istanbul_extra, proposer_seal_message, sig_hash,
	ConsensusState, FraudProof, HeaderVote, KaiaClientUpdate, KaiaCodecHeader, PendingParams,
	RatifiedParams,
};

/// Maximum number of headers a single client update may carry.
pub const MAX_HEADERS_PER_UPDATE: usize = 1024;

/// The outcome of a successful client update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
	pub height: u64,
	pub hash: H256,
	pub state_root: H256,
	pub timestamp: u64,
}

/// Verifies every header in the update against the evolving consensus state,
/// folding council and governance changes into `state`. The last header
/// becomes the new finalized state and is returned for the state commitment.
///
/// `state` is advanced header by header, so a rejected update leaves it
/// partially applied. Callers must therefore pass a state they can discard —
/// the ISMP client decodes a fresh copy per call and stores it only on
/// success, so a failed update never reaches storage.
pub fn verify_and_apply<H: Keccak256>(
	state: &mut ConsensusState,
	update: &KaiaClientUpdate,
) -> Result<VerificationResult, Error> {
	if update.headers.is_empty() {
		return Err(Error::EmptyUpdate);
	}
	if update.headers.len() > MAX_HEADERS_PER_UPDATE {
		return Err(Error::TooManyHeaders(MAX_HEADERS_PER_UPDATE));
	}
	state.validate()?;

	let mut result = None;
	let mut previous = None;
	for header in &update.headers {
		let number = header_number(header)?;
		if let Some(previous) = previous {
			if number <= previous {
				return Err(Error::UnorderedHeaders { header: number, previous });
			}
		} else if number <= state.finalized_height {
			return Err(Error::StaleHeader { header: number, finalized: state.finalized_height });
		}
		previous = Some(number);

		// Governance changes ratified at an epoch block activate at the next
		// epoch block; promote any that have matured before verifying seals,
		// since the committee size participates in the quorum calculation.
		if let Some(pending) = &state.pending_params {
			if number >= pending.activation_block {
				if let Some(size) = pending.committee_size {
					state.committee_size = size;
				}
				if let Some(node) = pending.governing_node {
					state.governing_node = node;
				}
				state.pending_params = None;
			}
		}

		let (hash, author) = verify_header_seals::<H>(state, header)?;

		// Validator votes take effect from the next block, so the council is
		// only updated after this header's seals have been verified.
		if !header.vote.is_empty() {
			apply_validator_vote(state, &header.vote, &author)?;
		}

		// Epoch blocks announce ratified governance parameters, effective
		// from the next epoch block.
		if number % state.epoch == 0 && !header.governance.is_empty() {
			let params = RatifiedParams::parse(&header.governance)?;
			if !params.is_empty() {
				state.pending_params = Some(PendingParams {
					activation_block: number.saturating_add(state.epoch),
					committee_size: params.committee_size,
					governing_node: params.governing_node,
				});
			}
		}

		state.finalized_height = number;
		state.finalized_hash = hash;
		result = Some(VerificationResult {
			height: number,
			hash,
			state_root: header.state_root,
			timestamp: header.timestamp,
		});
	}

	// Invariant: `headers` is non-empty, so a result was produced.
	result.ok_or(Error::EmptyUpdate)
}

/// Verifies a header's proposer seal and committed seals against the current
/// council without mutating any state. Returns the canonical block hash and
/// the recovered author.
pub fn verify_header_seals<H: Keccak256>(
	state: &ConsensusState,
	header: &KaiaCodecHeader,
) -> Result<(H256, H160), Error> {
	let (vanity, extra) = parse_istanbul_extra(&header.extra_data)?;

	// Each committed seal costs an `ecrecover`, and the seal list is covered by
	// no signature — both hash preimages strip it — so anyone may append to it
	// freely. The chain accepts a seal only from a council member and rejects
	// duplicates, so an honest header never carries more seals than there are
	// council members; bounding by that caps the work an update can force.
	if extra.committed_seals.len() > state.council.len() {
		return Err(Error::TooManySeals {
			found: extra.committed_seals.len() as u64,
			council: state.council.len() as u64,
		});
	}

	// The header declares the qualified validators, which are a subset of the
	// council on an honest chain. A declared validator the client has never
	// seen proves it skipped an `addvalidator` vote, leaving its council — and
	// therefore its quorum — understated. Reject rather than verify against a
	// set known to be stale; the field is inside both signed preimages, so a
	// proposer that fabricates it only invalidates its own block.
	for validator in &extra.validators {
		let validator = H160::from_slice(validator.as_slice());
		if !is_council_member(state, &validator) {
			return Err(Error::CouncilDrift { validator });
		}
	}

	let hash = header_hash::<H>(header, &vanity, &extra);
	let seal_message = proposer_seal_message::<H>(&sig_hash::<H>(header, &vanity, &extra));
	let author = recover_address::<H>(&extra.seal, &seal_message)?;
	if !is_council_member(state, &author) {
		return Err(Error::UnauthorizedAuthor);
	}

	let commit_message = committed_seal_message::<H>(&hash);
	let mut signers = BTreeSet::new();
	for seal in &extra.committed_seals {
		// Seals from addresses outside the tracked council are ignored rather
		// than rejected: the council here may lag the chain by a not-yet-seen
		// addvalidator vote, and quorum only ever counts known members.
		let Ok(signer) = recover_address::<H>(seal, &commit_message) else { continue };
		if is_council_member(state, &signer) {
			signers.insert(signer);
		}
	}

	let required = required_seals(state.council.len() as u64, state.committee_size);
	if (signers.len() as u64) < required {
		return Err(Error::InsufficientSeals { found: signers.len() as u64, required });
	}

	Ok((hash, author))
}

/// Checks two conflicting finalized headers at the same height, each carrying
/// a quorum of committed seals from the trusted council. Their existence
/// proves a BFT safety violation.
pub fn verify_fraud_proof<H: Keccak256>(
	state: &ConsensusState,
	proof_1: &FraudProof,
	proof_2: &FraudProof,
) -> Result<(), Error> {
	if header_number(&proof_1.header)? != header_number(&proof_2.header)? {
		return Err(Error::FraudProofHeightMismatch);
	}
	let (hash_1, _) = verify_header_seals::<H>(state, &proof_1.header)?;
	let (hash_2, _) = verify_header_seals::<H>(state, &proof_2.header)?;
	if hash_1 == hash_2 {
		return Err(Error::FraudProofIdenticalHeaders);
	}
	Ok(())
}

/// The number of distinct council committed seals required for finality,
/// mirroring the `2f + 1` import-time check in kaiachain/kaia with the
/// council size standing in for the qualified validator count.
pub fn required_seals(council_len: u64, committee_size: u64) -> u64 {
	let effective = council_len.min(committee_size);
	if effective == 0 {
		return u64::MAX; // an empty council can finalize nothing
	}
	let f = effective.div_ceil(3) - 1;
	2 * f + 1
}

/// Recovers the Ethereum-style address from a 65-byte `r || s || v` signature
/// over a 32-byte prehash.
pub fn recover_address<H: Keccak256>(signature: &[u8], prehash: &H256) -> Result<H160, Error> {
	if signature.len() != 65 {
		return Err(Error::InvalidSeal);
	}
	let mut sig = [0u8; 65];
	sig.copy_from_slice(signature);
	if sig[64] >= 27 {
		sig[64] -= 27;
	}
	let public_key =
		sp_io::crypto::secp256k1_ecdsa_recover(&sig, &prehash.0).map_err(|_| Error::InvalidSeal)?;
	let hash = H::keccak256(&public_key);
	Ok(H160::from_slice(&hash.0[12..]))
}

fn header_number(header: &KaiaCodecHeader) -> Result<u64, Error> {
	if header.number > U256::from(u64::MAX) {
		return Err(Error::InvalidExtraData);
	}
	Ok(header.number.low_u64())
}

fn is_council_member(state: &ConsensusState, address: &H160) -> bool {
	state.council.binary_search(address).is_ok()
}

/// Applies a validator vote to the council, effective for subsequent blocks.
/// The chain only finalizes votes cast by the block's proposer, and the
/// governing node can neither be added nor removed by vote.
fn apply_validator_vote(
	state: &mut ConsensusState,
	vote: &[u8],
	author: &H160,
) -> Result<(), Error> {
	let Some((voter, add, addresses)) = HeaderVote::parse_validator_vote(vote)? else {
		// Governance parameter votes are ratified via epoch blocks instead.
		return Ok(());
	};
	if voter != *author {
		return Err(Error::VoteNotFromAuthor);
	}
	for address in addresses {
		if address == state.governing_node {
			continue;
		}
		match state.council.binary_search(&address) {
			Ok(index) if !add => {
				state.council.remove(index);
			},
			Err(index) if add => {
				state.council.insert(index, address);
			},
			_ => {},
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloc::{vec, vec::Vec};
	use alloy_primitives::{Address, Bytes};
	use alloy_rlp::Encodable;
	use codec::Decode;
	use ethabi::ethereum_types::Bloom;
	use primitives::{canonicalize_validator_addresses, IstanbulExtra, ISTANBUL_EXTRA_VANITY};
	use sp_core::{ecdsa, Pair};

	struct TestHost;
	impl Keccak256 for TestHost {
		fn keccak256(bytes: &[u8]) -> H256 {
			sp_io::hashing::keccak_256(bytes).into()
		}
	}

	struct Signer {
		pair: ecdsa::Pair,
		address: H160,
	}

	fn signer(seed: u8) -> Signer {
		let pair = ecdsa::Pair::from_seed(&[seed; 32]);
		// Derive the Ethereum-style address by recovering our own signature
		// over a probe message; recovery is deterministic for a given key.
		let probe = [7u8; 32];
		let sig = pair.sign_prehashed(&probe);
		let address =
			recover_address::<TestHost>(&sig.0[..], &H256::from(probe)).expect("valid signature");
		Signer { pair, address }
	}

	fn sign(pair: &ecdsa::Pair, message: &H256) -> Bytes {
		pair.sign_prehashed(&message.0).0.to_vec().into()
	}

	fn encode_extra(validators: &[H160], seal: Bytes, committed_seals: Vec<Bytes>) -> Vec<u8> {
		let extra = IstanbulExtra {
			validators: validators.iter().map(|a| Address::from_slice(a.as_bytes())).collect(),
			seal,
			committed_seals,
		};
		let mut out = vec![0u8; ISTANBUL_EXTRA_VANITY];
		extra.encode(&mut out);
		out
	}

	fn base_header(number: u64) -> KaiaCodecHeader {
		KaiaCodecHeader {
			parent_hash: H256::repeat_byte(1),
			rewardbase: H160::repeat_byte(2),
			state_root: H256::repeat_byte(3),
			transactions_root: H256::repeat_byte(4),
			receipts_root: H256::repeat_byte(5),
			logs_bloom: Bloom::zero(),
			block_score: U256::one(),
			number: number.into(),
			gas_used: 21000,
			timestamp: 1_700_000_000 + number,
			timestamp_fos: 0,
			extra_data: vec![],
			governance: vec![],
			vote: vec![],
			base_fee_per_gas: Some(U256::from(25_000_000_000u64)),
			random_reveal: Some(vec![9u8; 96]),
			mix_hash: Some(vec![8u8; 32]),
			blob_gas_used: Some(0),
			excess_blob_gas: Some(0),
			vrank: None,
		}
	}

	/// Builds a fully sealed header: proposer seal from `signers[author]`,
	/// committed seals from `committers`. The declared validator list is left
	/// empty, which trivially satisfies the council-drift check; tests that
	/// exercise that check use [`sealed_header_declaring`].
	fn sealed_header(
		number: u64,
		signers: &[Signer],
		author: usize,
		committers: &[usize],
		vote: Vec<u8>,
	) -> KaiaCodecHeader {
		sealed_header_declaring(number, signers, author, committers, vote, &[])
	}

	fn sealed_header_declaring(
		number: u64,
		signers: &[Signer],
		author: usize,
		committers: &[usize],
		vote: Vec<u8>,
		validators: &[H160],
	) -> KaiaCodecHeader {
		let validators = validators.to_vec();
		let mut header = base_header(number);
		header.vote = vote;

		// The proposer seal signs the header before any seals are attached.
		header.extra_data = encode_extra(&validators, Bytes::new(), vec![]);
		let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		let sig_hash = sig_hash::<TestHost>(&header, &vanity, &extra);
		let seal = sign(&signers[author].pair, &proposer_seal_message::<TestHost>(&sig_hash));

		header.extra_data = encode_extra(&validators, seal.clone(), vec![]);
		let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		let hash = header_hash::<TestHost>(&header, &vanity, &extra);
		let commit_message = committed_seal_message::<TestHost>(&hash);
		let committed_seals = committers
			.iter()
			.map(|i| sign(&signers[*i].pair, &commit_message))
			.collect::<Vec<_>>();

		header.extra_data = encode_extra(&validators, seal, committed_seals);
		header
	}

	fn test_state(signers: &[Signer]) -> ConsensusState {
		let mut council = signers.iter().map(|s| s.address).collect::<Vec<_>>();
		council.sort();
		ConsensusState {
			council,
			governing_node: H160::repeat_byte(0xaa),
			epoch: 100,
			committee_size: 22,
			pending_params: None,
			finalized_height: 0,
			finalized_hash: H256::zero(),
			chain_id: 8217,
		}
	}

	fn signers(n: u8) -> Vec<Signer> {
		(1..=n).map(signer).collect()
	}

	#[test]
	fn accepts_header_with_quorum_seals() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		// council 33, committee 22 => f = 7, quorum = 15
		let committers = (0..15).collect::<Vec<_>>();
		let header = sealed_header(10, &signers, 0, &committers, vec![]);
		let update = KaiaClientUpdate { headers: vec![header.clone()] };

		let result = verify_and_apply::<TestHost>(&mut state, &update).unwrap();
		assert_eq!(result.height, 10);
		assert_eq!(result.state_root, header.state_root);
		assert_eq!(state.finalized_height, 10);
		assert_eq!(state.finalized_hash, result.hash);
	}

	#[test]
	fn rejects_header_below_quorum() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let committers = (0..14).collect::<Vec<_>>();
		let header = sealed_header(10, &signers, 0, &committers, vec![]);
		let update = KaiaClientUpdate { headers: vec![header] };

		let result = verify_and_apply::<TestHost>(&mut state, &update);
		assert_eq!(result, Err(Error::InsufficientSeals { found: 14, required: 15 }));
	}

	#[test]
	fn duplicate_seals_only_count_once() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		// 14 distinct committers with one duplicated seal: still below quorum.
		let committers = (0..14).chain(core::iter::once(13)).collect::<Vec<_>>();
		let header = sealed_header(10, &signers, 0, &committers, vec![]);
		let update = KaiaClientUpdate { headers: vec![header] };

		let result = verify_and_apply::<TestHost>(&mut state, &update);
		assert_eq!(result, Err(Error::InsufficientSeals { found: 14, required: 15 }));
	}

	#[test]
	fn rejects_author_outside_council() {
		let signers = signers(34);
		let (council_signers, stranger) = signers.split_at(33);
		let mut state = test_state(council_signers);
		let committers = (0..15).collect::<Vec<_>>();
		let header = sealed_header(10, &signers, 33, &committers, vec![]);
		let _ = stranger;
		let update = KaiaClientUpdate { headers: vec![header] };

		let result = verify_and_apply::<TestHost>(&mut state, &update);
		assert_eq!(result, Err(Error::UnauthorizedAuthor));
	}

	#[test]
	fn rejects_stale_header() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		state.finalized_height = 10;
		let header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vec![]);
		let update = KaiaClientUpdate { headers: vec![header] };

		let result = verify_and_apply::<TestHost>(&mut state, &update);
		assert_eq!(result, Err(Error::StaleHeader { header: 10, finalized: 10 }));
	}

	fn encode_vote(voter: H160, key: &[u8], addresses: &[H160]) -> Vec<u8> {
		let value = addresses.iter().flat_map(|a| a.as_bytes().to_vec()).collect::<Vec<u8>>();
		let vote = HeaderVote {
			voter: Address::from_slice(voter.as_bytes()),
			key: key.to_vec().into(),
			value: value.into(),
		};
		let mut out = vec![];
		vote.encode(&mut out);
		out
	}

	#[test]
	fn applies_add_and_remove_validator_votes() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let new_member = H160::repeat_byte(0x42);
		let committers = (0..15).collect::<Vec<_>>();

		// The author (proposer) votes to add a new council member.
		let vote = encode_vote(signers[0].address, primitives::ADD_VALIDATOR_KEY, &[new_member]);
		let header = sealed_header(10, &signers, 0, &committers, vote);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert!(state.council.binary_search(&new_member).is_ok());
		assert_eq!(state.council.len(), 34);

		// And later votes to remove them again.
		let vote = encode_vote(signers[0].address, primitives::REMOVE_VALIDATOR_KEY, &[new_member]);
		let header = sealed_header(11, &signers, 0, &committers, vote);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert!(state.council.binary_search(&new_member).is_err());
		assert_eq!(state.council.len(), 33);
	}

	#[test]
	fn rejects_vote_not_cast_by_author() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let vote = encode_vote(
			signers[5].address,
			primitives::ADD_VALIDATOR_KEY,
			&[H160::repeat_byte(0x42)],
		);
		let header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vote);

		let result =
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] });
		assert_eq!(result, Err(Error::VoteNotFromAuthor));
	}

	#[test]
	fn vote_cannot_remove_governing_node() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		state.governing_node = signers[1].address;
		let vote = encode_vote(
			signers[0].address,
			primitives::REMOVE_VALIDATOR_KEY,
			&[signers[1].address],
		);
		let header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vote);

		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert!(state.council.binary_search(&signers[1].address).is_ok());
	}

	#[test]
	fn ignores_non_validator_votes() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let vote = {
			let v = HeaderVote {
				voter: Address::from_slice(signers[0].address.as_bytes()),
				key: b"governance.unitprice".to_vec().into(),
				value: vec![1u8, 2, 3].into(),
			};
			let mut out = vec![];
			v.encode(&mut out);
			out
		};
		let header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vote);

		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert_eq!(state.council.len(), 33);
	}

	#[test]
	fn stages_and_promotes_committee_size_change() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let committers = (0..15).collect::<Vec<_>>();

		// Epoch block (epoch = 100) ratifying a committee size of 4.
		let mut header = sealed_header_with_governance(
			100,
			&signers,
			&committers,
			br#"{"istanbul.committeesize": 4}"#,
		);
		verify_and_apply::<TestHost>(
			&mut state,
			&KaiaClientUpdate { headers: vec![header.clone()] },
		)
		.unwrap();
		assert_eq!(
			state.pending_params,
			Some(PendingParams {
				activation_block: 200,
				committee_size: Some(4),
				governing_node: None
			})
		);
		assert_eq!(state.committee_size, 22);

		// Before activation the old quorum (15) still applies.
		header = sealed_header(150, &signers, 0, &committers, vec![]);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert_eq!(state.committee_size, 22);

		// From the activation block the new size (4 => f = 1, quorum = 3) applies.
		header = sealed_header(200, &signers, 0, &[0, 1, 2], vec![]);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert_eq!(state.committee_size, 4);
		assert_eq!(state.pending_params, None);
	}

	fn sealed_header_with_governance(
		number: u64,
		signers: &[Signer],
		committers: &[usize],
		governance_json: &[u8],
	) -> KaiaCodecHeader {
		let mut header = sealed_header(number, signers, 0, committers, vec![]);
		// governance = RLP byte-string wrapping the JSON object; the header
		// must be re-sealed since governance is part of the signed payload.
		let mut governance = vec![];
		Bytes::from(governance_json.to_vec()).encode(&mut governance);
		header.governance = governance;
		reseal(&mut header, signers, 0, committers);
		header
	}

	fn reseal(
		header: &mut KaiaCodecHeader,
		signers: &[Signer],
		author: usize,
		committers: &[usize],
	) {
		let validators = signers.iter().map(|s| s.address).collect::<Vec<_>>();
		header.extra_data = encode_extra(&validators, Bytes::new(), vec![]);
		let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		let sig_hash = sig_hash::<TestHost>(header, &vanity, &extra);
		let seal = sign(&signers[author].pair, &proposer_seal_message::<TestHost>(&sig_hash));

		header.extra_data = encode_extra(&validators, seal.clone(), vec![]);
		let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		let hash = header_hash::<TestHost>(header, &vanity, &extra);
		let commit_message = committed_seal_message::<TestHost>(&hash);
		let committed_seals = committers
			.iter()
			.map(|i| sign(&signers[*i].pair, &commit_message))
			.collect::<Vec<_>>();
		header.extra_data = encode_extra(&validators, seal, committed_seals);
	}

	#[test]
	fn detects_equivocation_fraud() {
		let signers = signers(33);
		let state = test_state(&signers);
		let committers = (0..15).collect::<Vec<_>>();
		let header_1 = sealed_header(10, &signers, 0, &committers, vec![]);
		let mut header_2 = sealed_header(10, &signers, 0, &committers, vec![]);
		header_2.state_root = H256::repeat_byte(0xde);
		reseal(&mut header_2, &signers, 0, &(0..15).collect::<Vec<_>>());

		verify_fraud_proof::<TestHost>(
			&state,
			&FraudProof { header: header_1.clone() },
			&FraudProof { header: header_2 },
		)
		.unwrap();

		// Identical headers are not fraud.
		let result = verify_fraud_proof::<TestHost>(
			&state,
			&FraudProof { header: header_1.clone() },
			&FraudProof { header: header_1 },
		);
		assert_eq!(result, Err(Error::FraudProofIdenticalHeaders));
	}

	#[test]
	fn quorum_math_matches_chain() {
		// mainnet: council 33, committee 22 => f = ceil(22/3) - 1 = 7 => 15
		assert_eq!(required_seals(33, 22), 15);
		// small qualified set: min(3, 22) = 3 => f = 0 => 1
		assert_eq!(required_seals(3, 22), 1);
		// committee of 21: f = 6 => 13
		assert_eq!(required_seals(33, 21), 13);
		assert_eq!(required_seals(0, 22), u64::MAX);
		// live mainnet today: council 33, committee size 50 => min is 33 => 21
		assert_eq!(required_seals(33, 50), 21);
	}

	/// Vote payloads taken verbatim from kaiachain/kaia
	/// `kaiax/valset/impl/getter_council_test.go`, with the address lists that
	/// the chain itself derives from them.
	#[test]
	fn parses_canonical_vote_vectors_from_the_chain() {
		let hex_vote = |s: &str| hex::decode(s).unwrap();
		let addr = |s: &str| H160::from_slice(&hex::decode(s).unwrap());

		// Not a validator vote (governance.unitprice), Kairos block 83863326.
		let vote = hex_vote("f09499fb17d324fa0e07f23b49d09028ac0919414db694676f7665726e616e63652e756e6974707269636585ae9f7bcc00");
		assert_eq!(HeaderVote::parse_validator_vote(&vote), Ok(None));

		// Kairos block 4202779: add one address.
		let vote = hex_vote("f8429499fb17d324fa0e07f23b49d09028ac0919414db697676f7665726e616e63652e61646476616c696461746f72948a88a093c05376886754a9b70b0d0a826a5e64be");
		let (voter, add, addresses) = HeaderVote::parse_validator_vote(&vote).unwrap().unwrap();
		assert_eq!(voter, addr("99fb17d324fa0e07f23b49d09028ac0919414db6"));
		assert!(add);
		assert_eq!(addresses, vec![addr("8a88a093c05376886754a9b70b0d0a826a5e64be")]);

		// Kairos block 4740968: remove one address.
		let vote = hex_vote("f8459499fb17d324fa0e07f23b49d09028ac0919414db69a676f7665726e616e63652e72656d6f766576616c696461746f72949419fa2e3b9eb1158de31be66c586a52f49c5de7");
		let (_, add, addresses) = HeaderVote::parse_validator_vote(&vote).unwrap().unwrap();
		assert!(!add);
		assert_eq!(addresses, vec![addr("9419fa2e3b9eb1158de31be66c586a52f49c5de7")]);

		// Mainnet block 75038594: an addvalidator whose value is the 42-byte
		// ASCII text of an address. Kaia does not hex-decode it, so the
		// council gains the address formed from the text's last 20 bytes —
		// and that address is in mainnet's canonical council to this day.
		let vote = hex_vote("f8589452d41ca72af615a1ac3301b0a93efa222ecc754197676f7665726e616e63652e61646476616c696461746f72aa307866386339633631633565376632623632313964316332386239346535636233636463383032353934");
		let (_, add, addresses) = HeaderVote::parse_validator_vote(&vote).unwrap().unwrap();
		assert!(add);
		assert_eq!(addresses, vec![addr("6332386239346535636233636463383032353934")]);

		// Mainnet block 90897408: the same shape, removing.
		let vote = hex_vote("f85b9452d41ca72af615a1ac3301b0a93efa222ecc75419a676f7665726e616e63652e72656d6f766576616c696461746f72aa307831366331393235383561306162323462353532373833623462663764386463396636383535633335");
		let (_, add, addresses) = HeaderVote::parse_validator_vote(&vote).unwrap().unwrap();
		assert!(!add);
		assert_eq!(addresses, vec![addr("3833623462663764386463396636383535633335")]);

		// Malformed RLP is the only rejected form.
		assert_eq!(
			HeaderVote::parse_validator_vote(&hex_vote("abcd")),
			Err(Error::InvalidVoteData)
		);
	}

	#[test]
	fn canonicalizes_vote_values_like_the_chain() {
		let a = H160::repeat_byte(0x11);
		let b = H160::repeat_byte(0x22);
		// Type 1 and 2: whole addresses.
		assert_eq!(canonicalize_validator_addresses(a.as_bytes()), vec![a]);
		let concat = [a.as_bytes(), b.as_bytes()].concat();
		assert_eq!(canonicalize_validator_addresses(&concat), vec![a, b]);
		// An empty value is a valid no-op vote, not an error.
		assert_eq!(canonicalize_validator_addresses(&[]), Vec::<H160>::new());
		// Type 3 over-length: the last 20 bytes.
		let long = [vec![0xff; 7], b.as_bytes().to_vec()].concat();
		assert_eq!(canonicalize_validator_addresses(&long), vec![b]);
		// Type 3 under-length: left-padded.
		assert_eq!(
			canonicalize_validator_addresses(&[0xab, 0xcd]),
			vec![H160::from_slice(&[&[0u8; 18][..], &[0xab, 0xcd]].concat())]
		);
	}

	/// A vote is applied only after the voting header is verified, so a newly
	/// added member cannot help seal the very block that admits them.
	#[test]
	fn added_member_seals_count_only_from_the_next_block() {
		let signers = signers(34);
		let (council_signers, newcomer) = signers.split_at(33);
		let mut state = test_state(council_signers);
		let newcomer_address = newcomer[0].address;

		// Block 10 admits the newcomer and includes their seal. That seal must
		// not count: 14 council seals plus the newcomer is still short of 15.
		let vote =
			encode_vote(signers[0].address, primitives::ADD_VALIDATOR_KEY, &[newcomer_address]);
		let committers = (0..14).chain(core::iter::once(33)).collect::<Vec<_>>();
		let header = sealed_header(10, &signers, 0, &committers, vote.clone());
		let result =
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] });
		assert_eq!(result, Err(Error::InsufficientSeals { found: 14, required: 15 }));

		// With a full council quorum the same header is accepted and the vote
		// applied, so from block 11 the newcomer's seal counts.
		let committers = (0..15).collect::<Vec<_>>();
		let header = sealed_header(10, &signers, 0, &committers, vote);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert!(state.council.binary_search(&newcomer_address).is_ok());

		let committers = (0..14).chain(core::iter::once(33)).collect::<Vec<_>>();
		let header = sealed_header(11, &signers, 0, &committers, vec![]);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
	}

	#[test]
	fn verifies_headers_from_a_later_round() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let mut header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vec![]);
		// A round change only alters the vanity's last byte, which both hash
		// preimages zero out, so seals stay valid.
		header.extra_data[ISTANBUL_EXTRA_VANITY - 1] = 3;

		let result =
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
				.unwrap();
		assert_eq!(result.height, 10);
	}

	#[test]
	fn promotes_pending_params_when_headers_skip_the_activation_block() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		state.pending_params = Some(PendingParams {
			activation_block: 200,
			committee_size: Some(4),
			governing_node: None,
		});

		// The first header after the activation block promotes it, even though
		// no header at exactly 200 was ever submitted.
		let header = sealed_header(250, &signers, 0, &[0, 1, 2], vec![]);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		assert_eq!(state.committee_size, 4);
		assert_eq!(state.pending_params, None);
	}

	#[test]
	fn stages_a_governing_node_change() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let new_node = H160::repeat_byte(0x5a);
		let json = alloc::format!(
			r#"{{"governance.governingnode": "{}"}}"#,
			hex::encode(new_node.as_bytes())
		);
		let header = sealed_header_with_governance(
			100,
			&signers,
			&(0..15).collect::<Vec<_>>(),
			json.as_bytes(),
		);

		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
		// Accepted without the 0x prefix, exactly as the chain canonicalizes it.
		assert_eq!(
			state.pending_params,
			Some(PendingParams {
				activation_block: 200,
				committee_size: None,
				governing_node: Some(new_node)
			})
		);
	}

	#[test]
	fn accepts_float_encoded_committee_size() {
		// The chain canonicalizes any float that round trips through u64.
		let params =
			RatifiedParams::parse(&rlp_json(br#"{"istanbul.committeesize": 22.0}"#)).unwrap();
		assert_eq!(params.committee_size, Some(22));
		let params =
			RatifiedParams::parse(&rlp_json(br#"{"istanbul.committeesize": 2.2e1}"#)).unwrap();
		assert_eq!(params.committee_size, Some(22));
		// A zero committee size is rejected by the chain's format checker, and
		// would make the quorum unsatisfiable here.
		assert_eq!(
			RatifiedParams::parse(&rlp_json(br#"{"istanbul.committeesize": 0}"#)),
			Err(Error::InvalidGovernanceData)
		);
		// Unrelated parameters are ignored rather than rejected.
		let params =
			RatifiedParams::parse(&rlp_json(br#"{"governance.unitprice": 25000000000}"#)).unwrap();
		assert!(params.is_empty());
	}

	fn rlp_json(json: &[u8]) -> Vec<u8> {
		let mut out = vec![];
		Bytes::from(json.to_vec()).encode(&mut out);
		out
	}

	#[test]
	fn rejects_out_of_order_and_malformed_updates() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let committers = (0..15).collect::<Vec<_>>();
		let first = sealed_header(11, &signers, 0, &committers, vec![]);
		let second = sealed_header(10, &signers, 0, &committers, vec![]);

		let result = verify_and_apply::<TestHost>(
			&mut state,
			&KaiaClientUpdate { headers: vec![first.clone(), second] },
		);
		assert_eq!(result, Err(Error::UnorderedHeaders { header: 10, previous: 11 }));

		// A repeated header is equally rejected. Note the update above left
		// `state` partially applied, so this starts from a fresh state.
		let mut state = test_state(&signers);
		let result = verify_and_apply::<TestHost>(
			&mut state,
			&KaiaClientUpdate { headers: vec![first.clone(), first] },
		);
		assert_eq!(result, Err(Error::UnorderedHeaders { header: 11, previous: 11 }));

		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![] }),
			Err(Error::EmptyUpdate)
		);

		// A zero epoch would divide by zero when testing for epoch blocks.
		let mut broken = test_state(&signers);
		broken.epoch = 0;
		let header = sealed_header(10, &signers, 0, &committers, vec![]);
		assert_eq!(
			verify_and_apply::<TestHost>(&mut broken, &KaiaClientUpdate { headers: vec![header] }),
			Err(Error::ZeroEpoch)
		);
	}

	#[test]
	fn rejects_malformed_extra_data_and_empty_seals() {
		let signers = signers(33);
		let mut state = test_state(&signers);

		let mut header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vec![]);
		header.extra_data.truncate(ISTANBUL_EXTRA_VANITY - 1);
		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] }),
			Err(Error::InvalidExtraData)
		);

		// No committed seals at all is simply a quorum failure.
		let header = sealed_header(10, &signers, 0, &[], vec![]);
		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] }),
			Err(Error::InsufficientSeals { found: 0, required: 15 })
		);
	}

	/// Seals that do not belong to the tracked council are ignored rather than
	/// rejected, since the council may legitimately lag a vote not yet seen.
	#[test]
	fn ignores_unknown_and_malformed_committed_seals() {
		let signers = signers(34);
		let mut state = test_state(&signers[..33]);
		let committers = (0..15).chain(core::iter::once(33)).collect::<Vec<_>>();
		let mut header = sealed_header(10, &signers, 0, &committers, vec![]);

		// Corrupt one seal's length so it cannot be recovered at all.
		let (vanity, mut extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		extra.committed_seals.push(Bytes::from(vec![0u8; 10]));
		let mut rebuilt = vanity.to_vec();
		extra.encode(&mut rebuilt);
		header.extra_data = rebuilt;

		let result =
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
				.unwrap();
		assert_eq!(result.height, 10);
	}

	#[test]
	fn consensus_state_round_trips_through_scale() {
		let signers = signers(4);
		let mut state = test_state(&signers);
		state.pending_params = Some(PendingParams {
			activation_block: 200,
			committee_size: Some(4),
			governing_node: Some(H160::repeat_byte(9)),
		});
		let decoded = ConsensusState::decode(&mut &codec::Encode::encode(&state)[..]).unwrap();
		assert_eq!(decoded, state);
	}

	#[test]
	fn rejects_oversized_updates() {
		let signers = signers(4);
		let mut state = test_state(&signers);
		let header = sealed_header(10, &signers, 0, &[0, 1, 2], vec![]);
		let headers = core::iter::repeat(header).take(MAX_HEADERS_PER_UPDATE + 1).collect();

		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers }),
			Err(Error::TooManyHeaders(MAX_HEADERS_PER_UPDATE))
		);
	}

	#[test]
	fn fraud_proof_requires_matching_heights() {
		let signers = signers(33);
		let state = test_state(&signers);
		let committers = (0..15).collect::<Vec<_>>();
		let header_1 = sealed_header(10, &signers, 0, &committers, vec![]);
		let header_2 = sealed_header(11, &signers, 0, &committers, vec![]);

		assert_eq!(
			verify_fraud_proof::<TestHost>(
				&state,
				&FraudProof { header: header_1 },
				&FraudProof { header: header_2 },
			),
			Err(Error::FraudProofHeightMismatch)
		);
	}

	/// A header declaring a validator the client has never seen proves an
	/// `addvalidator` vote was skipped, which would leave the quorum
	/// understated. It is rejected rather than verified against a stale set.
	#[test]
	fn rejects_headers_declaring_an_unknown_validator() {
		let signers = signers(34);
		let mut state = test_state(&signers[..33]);
		let declared = signers.iter().map(|s| s.address).collect::<Vec<_>>();
		let header = sealed_header_declaring(
			10,
			&signers,
			0,
			&(0..15).collect::<Vec<_>>(),
			vec![],
			&declared,
		);

		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] }),
			Err(Error::CouncilDrift { validator: signers[33].address })
		);

		// Declaring a subset of the council — the normal case, since demoted
		// members are omitted — is accepted.
		let mut state = test_state(&signers[..33]);
		let header = sealed_header_declaring(
			10,
			&signers,
			0,
			&(0..15).collect::<Vec<_>>(),
			vec![],
			&declared[..30],
		);
		verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] })
			.unwrap();
	}

	/// Committed seals are covered by no signature, so their number is bounded
	/// to cap the recovery work an update can force.
	#[test]
	fn rejects_more_seals_than_council_members() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let mut header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vec![]);

		let (vanity, mut extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		extra.committed_seals.extend((0..40).map(|_| Bytes::from(vec![0u8; 65])));
		let mut rebuilt = vanity.to_vec();
		extra.encode(&mut rebuilt);
		header.extra_data = rebuilt;

		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] }),
			Err(Error::TooManySeals { found: 55, council: 33 })
		);
	}

	/// Bytes appended after the Istanbul payload survive parsing but vanish
	/// when the header is re-encoded for hashing, so they are rejected.
	#[test]
	fn rejects_trailing_bytes_in_extra_data() {
		let signers = signers(33);
		let mut state = test_state(&signers);
		let mut header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vec![]);
		header.extra_data.extend_from_slice(&[0xde, 0xad]);

		assert_eq!(
			verify_and_apply::<TestHost>(&mut state, &KaiaClientUpdate { headers: vec![header] }),
			Err(Error::InvalidExtraData)
		);
	}

	#[test]
	fn rejects_malformed_consensus_states() {
		let signers = signers(33);
		let header = sealed_header(10, &signers, 0, &(0..15).collect::<Vec<_>>(), vec![]);
		let update = KaiaClientUpdate { headers: vec![header] };

		let mut zero_committee = test_state(&signers);
		zero_committee.committee_size = 0;
		assert_eq!(
			verify_and_apply::<TestHost>(&mut zero_committee, &update),
			Err(Error::ZeroCommitteeSize)
		);

		let mut empty_council = test_state(&signers);
		empty_council.council.clear();
		assert_eq!(
			verify_and_apply::<TestHost>(&mut empty_council, &update),
			Err(Error::MalformedCouncil)
		);

		// Membership is a binary search, so ordering is load bearing.
		let mut unsorted = test_state(&signers);
		unsorted.council.reverse();
		assert_eq!(
			verify_and_apply::<TestHost>(&mut unsorted, &update),
			Err(Error::MalformedCouncil)
		);

		let mut duplicated = test_state(&signers);
		duplicated.council.push(duplicated.council[0]);
		assert_eq!(
			verify_and_apply::<TestHost>(&mut duplicated, &update),
			Err(Error::MalformedCouncil)
		);
	}

	/// The same block sealed at two different rounds is not equivocation: the
	/// round is excluded from both hash preimages, so the hashes agree.
	#[test]
	fn round_differing_duplicates_are_not_fraud() {
		let signers = signers(33);
		let state = test_state(&signers);
		let committers = (0..15).collect::<Vec<_>>();
		let header_1 = sealed_header(10, &signers, 0, &committers, vec![]);
		let mut header_2 = header_1.clone();
		header_2.extra_data[ISTANBUL_EXTRA_VANITY - 1] = 5;

		assert_eq!(
			verify_fraud_proof::<TestHost>(
				&state,
				&FraudProof { header: header_1 },
				&FraudProof { header: header_2 },
			),
			Err(Error::FraudProofIdenticalHeaders)
		);
	}
}
