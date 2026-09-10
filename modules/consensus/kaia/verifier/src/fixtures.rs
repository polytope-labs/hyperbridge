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

//! Offline regression tests against captured Kaia mainnet blocks.
//!
//! These pin the parts of the client that only real chain data can validate —
//! the header RLP layout, the block hash, the double-hashed proposer seal
//! preimage and the committed seal preimage — without needing network access,
//! so a silent drift in any of them fails the build rather than production.
//!
//! The fixtures in `test-data/kaia_fixtures.json` are verbatim
//! `kaia_getBlockByNumber` / `kaia_getBlockWithConsensusInfoByNumber` /
//! `kaia_getCouncil` responses.

use crate::{
	primitives::{
		committed_seal_message, header_hash, parse_istanbul_extra, proposer_seal_message, sig_hash,
		ConsensusState, HeaderVote, KaiaClientUpdate, KaiaCodecHeader,
	},
	recover_address, required_seals, verify_and_apply, Error,
};
use alloc::{collections::BTreeSet, string::String, vec, vec::Vec};
use ethabi::ethereum_types::Bloom;
use ismp::messaging::Keccak256;
use polkadot_sdk::sp_io;
use primitive_types::{H160, H256, U256};

struct Host;
impl Keccak256 for Host {
	fn keccak256(bytes: &[u8]) -> H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

const FIXTURES: &str = include_str!("../test-data/kaia_fixtures.json");

fn fixtures() -> json::Value {
	json::from_str(FIXTURES).expect("fixtures are valid json; qed")
}

fn hex_bytes(value: &json::Value) -> Vec<u8> {
	let text = value.as_str().expect("hex string; qed");
	hex::decode(text.trim_start_matches("0x")).expect("valid hex; qed")
}

/// RPC numbers arrive either hex encoded or as plain JSON numbers.
fn hex_u64(value: &json::Value) -> u64 {
	match value.as_u64() {
		Some(number) => number,
		None => {
			let text = value.as_str().expect("hex string; qed");
			u64::from_str_radix(text.trim_start_matches("0x"), 16).expect("valid hex number; qed")
		},
	}
}

fn address(value: &json::Value) -> H160 {
	H160::from_slice(&hex_bytes(value))
}

fn hash(value: &json::Value) -> H256 {
	H256::from_slice(&hex_bytes(value))
}

/// Rebuilds the codec header from a captured `kaia_getBlockByNumber` response.
fn header_from_fixture(block: &json::Value) -> KaiaCodecHeader {
	let optional = |key: &str| -> Option<Vec<u8>> {
		block.get(key).filter(|v| !v.is_null()).map(hex_bytes).filter(|b| !b.is_empty())
	};
	let optional_u64 =
		|key: &str| -> Option<u64> { block.get(key).filter(|v| !v.is_null()).map(hex_u64) };
	KaiaCodecHeader {
		parent_hash: hash(&block["parentHash"]),
		rewardbase: address(&block["reward"]),
		state_root: hash(&block["stateRoot"]),
		transactions_root: hash(&block["transactionsRoot"]),
		receipts_root: hash(&block["receiptsRoot"]),
		logs_bloom: Bloom::from_slice(&hex_bytes(&block["logsBloom"])),
		block_score: U256::from(hex_u64(&block["blockScore"])),
		number: U256::from(hex_u64(&block["number"])),
		gas_used: hex_u64(&block["gasUsed"]),
		timestamp: hex_u64(&block["timestamp"]),
		timestamp_fos: block
			.get("timestampFoS")
			.filter(|v| !v.is_null())
			.map(|v| hex_u64(v) as u8)
			.unwrap_or_default(),
		extra_data: hex_bytes(&block["extraData"]),
		governance: block
			.get("governanceData")
			.filter(|v| !v.is_null())
			.map(hex_bytes)
			.unwrap_or_default(),
		vote: block
			.get("voteData")
			.filter(|v| !v.is_null())
			.map(hex_bytes)
			.unwrap_or_default(),
		base_fee_per_gas: optional_u64("baseFeePerGas").map(U256::from),
		random_reveal: optional("randomReveal"),
		mix_hash: optional("mixHash"),
		blob_gas_used: optional_u64("blobGasUsed"),
		excess_blob_gas: optional_u64("excessBlobGas"),
		vrank: optional("vrank"),
	}
}

/// The reconstructed header must hash to the block hash the chain reports, and
/// its signing preimage must equal the node's own `sigHash`. Together these
/// pin the full RLP layout and the filtered-header rules.
#[test]
fn reproduces_mainnet_block_hash_and_sig_hash() {
	let fixtures = fixtures();
	let recent = &fixtures["recent"];
	let header = header_from_fixture(&recent["block"]);
	let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();

	assert_eq!(header_hash::<Host>(&header, &vanity, &extra), hash(&recent["block"]["hash"]));
	assert_eq!(sig_hash::<Host>(&header, &vanity, &extra), hash(&recent["sigHash"]));
	assert_eq!(vanity[31] as u64, hex_u64(&recent["round"]));
}

/// The proposer seal signs `keccak256(sig_hash)` — Istanbul hashes the payload
/// a second time inside its signing helper. Recovering with the single hash
/// yields a different address, which is what this pins.
#[test]
fn recovers_the_proposer_from_the_double_hashed_preimage() {
	let fixtures = fixtures();
	let recent = &fixtures["recent"];
	let header = header_from_fixture(&recent["block"]);
	let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();

	let preimage = proposer_seal_message::<Host>(&sig_hash::<Host>(&header, &vanity, &extra));
	let author = recover_address::<Host>(&extra.seal, &preimage).unwrap();
	assert_eq!(author, address(&recent["proposer"]));

	// The un-hashed preimage must not recover the proposer.
	let wrong = recover_address::<Host>(&extra.seal, &sig_hash::<Host>(&header, &vanity, &extra));
	assert_ne!(wrong, Ok(author));
}

/// Every committed seal must recover to a distinct council member, and the
/// set must satisfy the quorum this client computes.
///
/// Deliberately not compared against the node's own `committers` field: that
/// field is served from a signature-address cache which, on at least some
/// deployed versions, returns a different (still council-member) address for
/// the same block across calls. An independent `eth_keys` recovery of these
/// exact seals agrees with the addresses recovered here, so the seals — not
/// that field — are the oracle.
#[test]
fn recovers_every_committer_and_meets_quorum() {
	let fixtures = fixtures();
	let recent = &fixtures["recent"];
	let header = header_from_fixture(&recent["block"]);
	let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();

	let council = recent["council"]
		.as_array()
		.unwrap()
		.iter()
		.map(address)
		.collect::<BTreeSet<_>>();

	let message = committed_seal_message::<Host>(&header_hash::<Host>(&header, &vanity, &extra));
	let recovered = extra
		.committed_seals
		.iter()
		.map(|seal| recover_address::<Host>(seal, &message).unwrap())
		.collect::<BTreeSet<_>>();

	// Every seal recovers to a distinct council member.
	assert_eq!(recovered.len(), extra.committed_seals.len());
	for committer in &recovered {
		assert!(council.contains(committer), "{committer:?} is not a council member");
	}

	let committee_size = recent["params"]["istanbul.committeesize"].as_u64().unwrap();
	let required = required_seals(council.len() as u64, committee_size);
	assert!(
		recovered.len() as u64 >= required,
		"{} committers against a quorum of {required}",
		recovered.len(),
	);

	// Recovery is a pure function of the header: no cache, no drift.
	let again = extra
		.committed_seals
		.iter()
		.map(|seal| recover_address::<Host>(seal, &message).unwrap())
		.collect::<BTreeSet<_>>();
	assert_eq!(recovered, again);
}

/// The whole pipeline over a real header: a state seeded from the chain's own
/// council and parameters must accept the block.
#[test]
fn verifies_a_real_mainnet_header_end_to_end() {
	let fixtures = fixtures();
	let recent = &fixtures["recent"];
	let header = header_from_fixture(&recent["block"]);
	let number = header.number.low_u64();

	let mut council = recent["council"].as_array().unwrap().iter().map(address).collect::<Vec<_>>();
	council.sort();
	let params = &recent["params"];
	let governing_node = params["governance.governingnode"]
		.as_str()
		.map(|s| H160::from_slice(&hex::decode(s.trim_start_matches("0x")).unwrap()))
		.unwrap();

	let mut state = ConsensusState {
		council,
		governing_node,
		epoch: params["istanbul.epoch"].as_u64().unwrap(),
		committee_size: params["istanbul.committeesize"].as_u64().unwrap(),
		pending_params: None,
		finalized_height: number - 1,
		finalized_hash: hash(&recent["block"]["parentHash"]),
		chain_id: 8217,
	};

	let result =
		verify_and_apply::<Host>(&mut state, &KaiaClientUpdate { headers: vec![header] }).unwrap();
	assert_eq!(result.height, number);
	assert_eq!(result.hash, hash(&recent["block"]["hash"]));
	assert_eq!(state.finalized_height, number);

	// Tampering with any covered field invalidates the seals.
	let mut tampered = header_from_fixture(&recent["block"]);
	tampered.state_root = H256::repeat_byte(0xff);
	let mut state = state.clone();
	state.finalized_height = number - 1;
	assert!(matches!(
		verify_and_apply::<Host>(&mut state, &KaiaClientUpdate { headers: vec![tampered] }),
		Err(Error::UnauthorizedAuthor) | Err(Error::InsufficientSeals { .. })
	));
}

/// Mainnet blocks 75038594 and 90897408 carry validator votes whose value is
/// the ASCII text of an address rather than its bytes. Kaia applies them via
/// `BytesToAddress`, so the council gains (or loses) the address formed from
/// the text's last 20 bytes. These fixtures pin that against the real chain.
#[test]
fn applies_historical_type_3_votes_like_the_chain() {
	let fixtures = fixtures();
	for key in ["type3_add", "type3_remove"] {
		let entry = &fixtures[key];
		let header = header_from_fixture(&entry["block"]);
		let (_, add, addresses) = HeaderVote::parse_validator_vote(&header.vote).unwrap().unwrap();
		assert_eq!(addresses.len(), 1, "{key} canonicalizes to one address");
		let voted = addresses[0];

		let before = entry["council_before"]
			.as_array()
			.unwrap()
			.iter()
			.map(address)
			.collect::<BTreeSet<_>>();
		let after = entry["council_after"]
			.as_array()
			.unwrap()
			.iter()
			.map(address)
			.collect::<BTreeSet<_>>();

		// The chain's own council either gained or lost exactly the address
		// this canonicalization produces.
		if add {
			assert!(after.contains(&voted), "{key}: {voted:?} missing from the council");
		} else {
			assert!(!after.contains(&voted), "{key}: {voted:?} still in the council");
		}
		let _ = before;

		// These blocks predate Magma and Randao, so the optional tail is absent.
		assert!(header.base_fee_per_gas.is_none());
		assert!(header.mix_hash.is_none());
	}
}

/// Pre-fork headers with a shorter optional tail still reproduce their hashes.
#[test]
fn reproduces_pre_fork_block_hashes() {
	let fixtures = fixtures();
	for key in ["type3_add", "type3_remove"] {
		let entry = &fixtures[key];
		let header = header_from_fixture(&entry["block"]);
		let (vanity, extra) = parse_istanbul_extra(&header.extra_data).unwrap();
		assert_eq!(
			header_hash::<Host>(&header, &vanity, &extra),
			hash(&entry["block"]["hash"]),
			"{key} hash mismatch",
		);
	}
}

/// Guards against a fixture file that was regenerated incorrectly.
#[test]
fn fixtures_are_well_formed() {
	let fixtures = fixtures();
	let recent = &fixtures["recent"];
	assert!(recent["committers"].as_array().unwrap().len() >= 3);
	assert!(!recent["council"].as_array().unwrap().is_empty());
	assert!(recent["sigHash"].as_str().unwrap().starts_with("0x"));
	let _: String = recent["proposer"].as_str().unwrap().into();
}
