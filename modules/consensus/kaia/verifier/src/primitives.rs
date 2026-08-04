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

//! Types and codecs for Kaia block headers and Istanbul BFT consensus metadata.

use alloc::{vec, vec::Vec};
use alloy_primitives::{Address, Bloom as AlloyBloom, Bytes, B256, U256 as AlloyU256};
use alloy_rlp::{Decodable, Encodable};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use ethabi::ethereum_types::Bloom;
use ismp::messaging::Keccak256;
use primitive_types::{H160, H256, U256};

use crate::error::Error;

/// Number of bytes reserved at the start of `extra_data` for proposer vanity.
/// The last byte of the vanity area carries the consensus round number.
pub const ISTANBUL_EXTRA_VANITY: usize = 32;

/// Message code appended to the proposal hash when signing a COMMIT message.
/// Must stay in sync with Kaia's `bft.MsgCommit`.
pub const MSG_COMMIT_CODE: u8 = 2;

/// Governance vote key for adding validators to the council.
pub const ADD_VALIDATOR_KEY: &[u8] = b"governance.addvalidator";

/// Governance vote key for removing validators from the council.
pub const REMOVE_VALIDATOR_KEY: &[u8] = b"governance.removevalidator";

/// Governance parameter name for the committee size (`istanbul.committeesize`).
pub const COMMITTEE_SIZE_PARAM: &str = "istanbul.committeesize";

/// Governance parameter name for the governing node (`governance.governingnode`).
pub const GOVERNING_NODE_PARAM: &str = "governance.governingnode";

/// Kaia mainnet EVM chain id.
pub const KAIA_MAINNET_CHAIN_ID: u32 = 8217;

/// Kaia Kairos testnet EVM chain id.
pub const KAIA_TESTNET_CHAIN_ID: u32 = 1001;

/// A Kaia block header in SCALE-friendly form. Field order mirrors the RLP
/// encoding in kaiachain/kaia `blockchain/types/block.go`. Kaia headers carry
/// no uncle hash, difficulty, gas limit, or nonce; they add `rewardbase`,
/// `block_score`, `timestamp_fos`, `governance` and `vote`.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct KaiaCodecHeader {
	pub parent_hash: H256,
	pub rewardbase: H160,
	pub state_root: H256,
	pub transactions_root: H256,
	pub receipts_root: H256,
	pub logs_bloom: Bloom,
	pub block_score: U256,
	pub number: U256,
	pub gas_used: u64,
	pub timestamp: u64,
	/// Fraction of a second since `timestamp`.
	pub timestamp_fos: u8,
	pub extra_data: Vec<u8>,
	/// RLP-wrapped JSON of governance parameters ratified at this (epoch) block.
	pub governance: Vec<u8>,
	/// RLP-encoded governance vote cast by this block's proposer, if any.
	pub vote: Vec<u8>,
	// Optional tail, in RLP order. Kaia only ever appends new header fields.
	pub base_fee_per_gas: Option<U256>,
	/// 96-byte BLS signature over the block number (KIP-114), post-Randao.
	pub random_reveal: Option<Vec<u8>>,
	/// 32-byte RANDAO mix, post-Randao.
	pub mix_hash: Option<Vec<u8>>,
	pub blob_gas_used: Option<u64>,
	pub excess_blob_gas: Option<u64>,
	/// KIP-227 VRank data, post-permissionless.
	pub vrank: Option<Vec<u8>>,
}

/// The RLP wire form of [`KaiaCodecHeader`]. Trailing `Option` fields are
/// omitted from the encoding when `None`, matching Go's `rlp:"optional"`.
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct KaiaHeader {
	pub parent_hash: B256,
	pub rewardbase: Address,
	pub state_root: B256,
	pub transactions_root: B256,
	pub receipts_root: B256,
	pub logs_bloom: AlloyBloom,
	pub block_score: AlloyU256,
	pub number: AlloyU256,
	pub gas_used: u64,
	pub timestamp: AlloyU256,
	pub timestamp_fos: u8,
	pub extra_data: Bytes,
	pub governance: Bytes,
	pub vote: Bytes,
	pub base_fee_per_gas: Option<AlloyU256>,
	pub random_reveal: Option<Bytes>,
	pub mix_hash: Option<Bytes>,
	pub blob_gas_used: Option<u64>,
	pub excess_blob_gas: Option<u64>,
	pub vrank: Option<Bytes>,
}

impl From<&KaiaCodecHeader> for KaiaHeader {
	fn from(header: &KaiaCodecHeader) -> Self {
		KaiaHeader {
			parent_hash: B256::from_slice(&header.parent_hash[..]),
			rewardbase: Address::from_slice(&header.rewardbase[..]),
			state_root: B256::from_slice(&header.state_root[..]),
			transactions_root: B256::from_slice(&header.transactions_root[..]),
			receipts_root: B256::from_slice(&header.receipts_root[..]),
			logs_bloom: AlloyBloom::from_slice(header.logs_bloom.as_bytes()),
			block_score: AlloyU256::from_limbs(header.block_score.0),
			number: AlloyU256::from_limbs(header.number.0),
			gas_used: header.gas_used,
			timestamp: AlloyU256::from(header.timestamp),
			timestamp_fos: header.timestamp_fos,
			extra_data: header.extra_data.clone().into(),
			governance: header.governance.clone().into(),
			vote: header.vote.clone().into(),
			base_fee_per_gas: header.base_fee_per_gas.map(|b| AlloyU256::from_limbs(b.0)),
			random_reveal: header.random_reveal.clone().map(Into::into),
			mix_hash: header.mix_hash.clone().map(Into::into),
			blob_gas_used: header.blob_gas_used,
			excess_blob_gas: header.excess_blob_gas,
			vrank: header.vrank.clone().map(Into::into),
		}
	}
}

/// Istanbul consensus metadata carried in `extra_data` after the 32-byte
/// vanity prefix.
#[derive(RlpDecodable, RlpEncodable, Debug, Clone, PartialEq, Eq)]
pub struct IstanbulExtra {
	/// The qualified validator set as claimed by the proposer.
	pub validators: Vec<Address>,
	/// The proposer's secp256k1 seal over `sig_hash`.
	pub seal: Bytes,
	/// Committee members' secp256k1 seals over `keccak256(header_hash || 0x02)`.
	pub committed_seals: Vec<Bytes>,
}

/// Parses `extra_data` into the vanity prefix and Istanbul consensus metadata.
pub fn parse_istanbul_extra(extra_data: &[u8]) -> Result<([u8; 32], IstanbulExtra), Error> {
	if extra_data.len() < ISTANBUL_EXTRA_VANITY {
		return Err(Error::InvalidExtraData);
	}
	let mut vanity = [0u8; ISTANBUL_EXTRA_VANITY];
	vanity.copy_from_slice(&extra_data[..ISTANBUL_EXTRA_VANITY]);
	// Kaia's `rlp.DecodeBytes` rejects trailing input, so this does too:
	// bytes past the payload survive parsing but are erased by the canonical
	// re-encoding used for hashing, which would let one block hash stand for
	// many distinct headers.
	let mut payload = &extra_data[ISTANBUL_EXTRA_VANITY..];
	let extra = IstanbulExtra::decode(&mut payload).map_err(|_| Error::InvalidExtraData)?;
	if !payload.is_empty() {
		return Err(Error::InvalidExtraData);
	}
	Ok((vanity, extra))
}

/// Rebuilds `extra_data` with the round byte zeroed, committed seals stripped
/// and, when `keep_seal` is false, the proposer seal stripped as well. This
/// mirrors `istanbulFilteredHeader` in kaiachain/kaia.
fn filtered_extra_data(vanity: &[u8; 32], extra: &IstanbulExtra, keep_seal: bool) -> Vec<u8> {
	let filtered = IstanbulExtra {
		validators: extra.validators.clone(),
		seal: if keep_seal { extra.seal.clone() } else { Bytes::new() },
		committed_seals: vec![],
	};
	let mut vanity = *vanity;
	vanity[ISTANBUL_EXTRA_VANITY - 1] = 0;
	let mut out = vanity.to_vec();
	filtered.encode(&mut out);
	out
}

fn filtered_header_rlp(
	header: &KaiaCodecHeader,
	vanity: &[u8; 32],
	extra: &IstanbulExtra,
	keep_seal: bool,
) -> Vec<u8> {
	let mut rlp_header = KaiaHeader::from(header);
	rlp_header.extra_data = filtered_extra_data(vanity, extra, keep_seal).into();
	alloy_rlp::encode(&rlp_header)
}

/// The canonical Kaia block hash: the header with the round byte zeroed and
/// committed seals stripped, but the proposer seal kept.
pub fn header_hash<H: Keccak256>(
	header: &KaiaCodecHeader,
	vanity: &[u8; 32],
	extra: &IstanbulExtra,
) -> H256 {
	H::keccak256(&filtered_header_rlp(header, vanity, extra, true))
}

/// The preimage hash signed by the proposer: the header with the round byte
/// zeroed and both the proposer seal and committed seals stripped.
pub fn sig_hash<H: Keccak256>(
	header: &KaiaCodecHeader,
	vanity: &[u8; 32],
	extra: &IstanbulExtra,
) -> H256 {
	H::keccak256(&filtered_header_rlp(header, vanity, extra, false))
}

/// The message hash signed by committee members in their committed seals.
pub fn committed_seal_message<H: Keccak256>(header_hash: &H256) -> H256 {
	let mut buf = header_hash.as_bytes().to_vec();
	buf.push(MSG_COMMIT_CODE);
	H::keccak256(&buf)
}

/// The message hash signed by the proposer in its seal. Istanbul's signature
/// convention (`GetSignatureAddress`) hashes the payload — here the 32-byte
/// `sig_hash` — once more before signing.
pub fn proposer_seal_message<H: Keccak256>(sig_hash: &H256) -> H256 {
	H::keccak256(sig_hash.as_bytes())
}

/// A governance vote inscribed in a block header's `vote` field,
/// RLP-encoded as `[voter, key, value]`.
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
pub struct HeaderVote {
	pub voter: Address,
	pub key: Bytes,
	pub value: Bytes,
}

impl HeaderVote {
	/// Decodes the vote and, if it is a validator vote, returns the voter,
	/// whether it adds (true) or removes (false), and the target addresses.
	/// Non-validator votes (governance parameter votes) return `None`.
	pub fn parse_validator_vote(vote: &[u8]) -> Result<Option<(H160, bool, Vec<H160>)>, Error> {
		let decoded = HeaderVote::decode(&mut &vote[..]).map_err(|_| Error::InvalidVoteData)?;
		let add = match &*decoded.key {
			key if key == ADD_VALIDATOR_KEY => true,
			key if key == REMOVE_VALIDATOR_KEY => false,
			_ => return Ok(None),
		};
		Ok(Some((
			H160::from_slice(decoded.voter.as_slice()),
			add,
			canonicalize_validator_addresses(&decoded.value),
		)))
	}
}

/// Canonicalizes a validator vote's value into the address list the chain
/// applies, mirroring `validatorAddressListCanonicalizer` in kaiachain/kaia
/// `kaiax/gov/param.go`. The chain never rejects a value on shape (validator
/// params use a no-op format checker), so neither may this.
///
/// Three encodings occur in practice:
/// - a single 20-byte address,
/// - `20 * n` concatenated addresses (`n == 0` is a valid no-op),
/// - any other length: one address via `BytesToAddress`, i.e. the last 20
///   bytes, left-padded with zeros when shorter. Kaia deliberately does *not*
///   hex-decode this form, so a vote carrying the ASCII text of an address
///   installs the address formed from that text's final 20 bytes. Mainnet
///   blocks 75038594 and 90897408 did exactly that, and the resulting address
///   is part of the canonical council.
pub fn canonicalize_validator_addresses(value: &[u8]) -> Vec<H160> {
	if value.len() % 20 == 0 {
		return value.chunks(20).map(H160::from_slice).collect();
	}
	let mut address = [0u8; 20];
	if value.len() > 20 {
		address.copy_from_slice(&value[value.len() - 20..]);
	} else {
		address[20 - value.len()..].copy_from_slice(value);
	}
	vec![H160::from(address)]
}

/// Governance parameters ratified at an epoch block that this client tracks.
/// Parsed from the header's `governance` field: an RLP byte-string wrapping
/// a JSON object of `{parameter name: value}`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RatifiedParams {
	pub committee_size: Option<u64>,
	pub governing_node: Option<H160>,
}

impl RatifiedParams {
	pub fn parse(governance: &[u8]) -> Result<Self, Error> {
		let json_bytes =
			Bytes::decode(&mut &governance[..]).map_err(|_| Error::InvalidGovernanceData)?;
		let params: json::Map<alloc::string::String, json::Value> =
			json::from_slice(&json_bytes).map_err(|_| Error::InvalidGovernanceData)?;

		let committee_size = match params.get(COMMITTEE_SIZE_PARAM) {
			Some(value) => Some(canonicalize_u64(value).ok_or(Error::InvalidGovernanceData)?),
			None => None,
		};
		let governing_node = match params.get(GOVERNING_NODE_PARAM) {
			Some(value) => Some(canonicalize_address(value).ok_or(Error::InvalidGovernanceData)?),
			None => None,
		};
		Ok(RatifiedParams { committee_size, governing_node })
	}

	pub fn is_empty(&self) -> bool {
		self.committee_size.is_none() && self.governing_node.is_none()
	}
}

/// Accepts the same numeric domain as Kaia's `uint64Canonicalizer`: JSON
/// integers, and floats whose value survives a round trip through `u64`
/// (`22.0` and `2.2e1` are both valid encodings of 22 on-chain). The committee
/// size additionally must be positive, matching Kaia's format checker — a zero
/// would otherwise make the quorum unsatisfiable.
fn canonicalize_u64(value: &json::Value) -> Option<u64> {
	let out = match value.as_u64() {
		Some(int) => int,
		None => {
			let float = value.as_f64()?;
			if !float.is_finite() || float < 0.0 || float > u64::MAX as f64 {
				return None;
			}
			let int = float as u64;
			if int as f64 != float {
				return None;
			}
			int
		},
	};
	(out > 0).then_some(out)
}

/// Accepts the same address domain as Kaia's `addressCanonicalizer`, which
/// uses `common.IsHexAddress`: 40 hex digits with or without the `0x` prefix.
fn canonicalize_address(value: &json::Value) -> Option<H160> {
	let text = value.as_str()?;
	let digits = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")).unwrap_or(text);
	let bytes = const_hex::decode(digits).ok()?;
	(bytes.len() == 20).then(|| H160::from_slice(&bytes))
}

// alloy-primitives re-exports const-hex as its hex facade; use it for the
// governing node address without pulling in another hex crate.
use alloy_primitives::hex as const_hex;

/// Governance parameter changes staged for activation at a future epoch block.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct PendingParams {
	/// Block number from which the changes apply.
	pub activation_block: u64,
	pub committee_size: Option<u64>,
	pub governing_node: Option<H160>,
}

/// The Kaia consensus client state.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct ConsensusState {
	/// Current council members, sorted ascending. Committed seals are counted
	/// against this set.
	pub council: Vec<H160>,
	/// The governing node (`governance.governingnode`). Exempt from validator
	/// votes: it can neither be added to nor removed from the council by vote.
	pub governing_node: H160,
	/// Header governance epoch in blocks (`istanbul.epoch`, 604800 on mainnet).
	pub epoch: u64,
	/// Current committee size (`istanbul.committeesize`, 22 on mainnet).
	pub committee_size: u64,
	/// Committee size / governing node changes ratified at the last processed
	/// epoch block, effective from `activation_block`.
	pub pending_params: Option<PendingParams>,
	/// Height of the latest verified finalized header.
	pub finalized_height: u64,
	/// Hash of the latest verified finalized header.
	pub finalized_hash: H256,
	/// The EVM chain id of this Kaia network.
	pub chain_id: u32,
}

impl ConsensusState {
	/// Rejects a state that would make verification unsound or trap the
	/// runtime. Nothing validates the state at creation — `create_consensus_client`
	/// stores the bytes verbatim — so a malformed one would otherwise surface
	/// as a division by zero or a silently under-counted quorum.
	pub fn validate(&self) -> Result<(), Error> {
		if self.epoch == 0 {
			return Err(Error::ZeroEpoch);
		}
		if self.committee_size == 0 {
			return Err(Error::ZeroCommitteeSize);
		}
		// Membership is a binary search, so an unsorted or duplicated council
		// would silently fail to match seals that ought to count.
		if self.council.is_empty() || self.council.windows(2).any(|pair| pair[0] >= pair[1]) {
			return Err(Error::MalformedCouncil);
		}
		Ok(())
	}
}

/// A consensus proof for the Kaia consensus client: one or more finalized
/// headers in strictly ascending order. Every header that contains a
/// validator vote or epoch governance data since the last finalized height
/// must be included so the client can track council and parameter changes;
/// the last header becomes the new finalized state.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct KaiaClientUpdate {
	pub headers: Vec<KaiaCodecHeader>,
}

/// A fraud proof: two distinct headers at the same height, both carrying a
/// quorum of committed seals from the trusted council.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct FraudProof {
	pub header: KaiaCodecHeader,
}
