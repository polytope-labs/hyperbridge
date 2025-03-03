// Copyright (C) 2023 PolytopeLabs.
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
#![allow(clippy::all)]
#![deny(missing_docs)]

//! GRANDPA consensus prover utilities

use anyhow::anyhow;
use codec::{Decode, Encode};
use finality_grandpa::Chain as _;
use grandpa_verifier_primitives::{
	justification::{find_scheduled_change, AncestryChain},
	parachain_header_storage_key, ConsensusState, DefaultHeader, FinalityProof,
	ParachainHeaderProofs,
};
use indicatif::ProgressBar;
use ismp::host::StateMachine;
use polkadot_sdk::{sp_consensus_grandpa::GRANDPA_ENGINE_ID, *};
use serde::{Deserialize, Serialize};
use sp_consensus_grandpa::{AuthorityId, AuthoritySignature};
use sp_core::H256;
use sp_runtime::traits::{One, Zero};
use std::collections::{BTreeMap, BTreeSet};
use subxt::{config::Header, rpc_params, Config, OnlineClient};

/// Head data for parachain
#[derive(Decode, Encode)]
pub struct HeadData(pub Vec<u8>);

/// Contains methods useful for proving parachain and standalone-chain header finality using GRANDPA
#[derive(Clone)]
pub struct GrandpaProver<T: Config> {
	/// Subxt client for the chain
	pub client: OnlineClient<T>,
	/// Options for the prover
	pub options: ProverOptions,
}

/// We redefine these here because we want the header to be bounded by subxt::config::Header in the
/// prover
pub type Commit = finality_grandpa::Commit<H256, u32, AuthoritySignature, AuthorityId>;

/// This is the storage key for the grandpa.currentSetId storage item in the runtime. Ideally the
/// grandpa pallet is always referred to as "grandpa" in the construct runtime macro.
pub const GRANDPA_CURRENT_SET_ID: [u8; 32] =
	hex_literal::hex!("5f9cc45b7a00c5899361e1c6099678dc8a2d09463effcc78a22d75b9cb87dffc");

/// Justification
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Encode, Decode)]
pub struct GrandpaJustification<H: Header + codec::Decode> {
	/// Current voting round number, monotonically increasing
	pub round: u64,
	/// Contains block hash & number that's being finalized and the signatures.
	pub commit: Commit,
	/// Contains the path from a [`PreCommit`]'s target hash to the GHOST finalized block.
	pub votes_ancestries: Vec<H>,
}

/// Options for initializing the GRANDPA consensus prover.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProverOptions {
	/// The ws url to the node
	pub ws_url: String,
	/// Parachain Ids if this GRANDPA consensus hosts parachains
	pub para_ids: Vec<u32>,
	/// State machine identifier for the chain
	pub state_machine: StateMachine,
	/// Max rpc payload for websocket connections
	pub max_rpc_payload_size: u32,
	/// Maximum block range to prove finality for
	pub max_block_range: u32,
}

/// An encoded justification proving that the given header has been finalized
#[derive(Clone, Serialize, Deserialize)]
pub struct JustificationNotification(pub sp_core::Bytes);

impl<T> GrandpaProver<T>
where
	T: Config,
	<T::Header as Header>::Number: Ord + Zero,
	u32: From<<T::Header as Header>::Number>,
	sp_core::H256: From<T::Hash>,
	T::Header: codec::Decode,
{
	/// Initializes the GRANDPA prover given the parameters. Internally connects over WS to the
	/// provided RPC
	pub async fn new(options: ProverOptions) -> Result<Self, anyhow::Error> {
		let ProverOptions { max_rpc_payload_size, ref ws_url, .. } = options;
		let client = subxt_utils::client::ws_client(&ws_url, max_rpc_payload_size).await?;

		Ok(Self { client, options })
	}

	/// Construct the initial consensus state.
	pub async fn initialize_consensus_state(
		&self,
		slot_duration: u64,
		hash: T::Hash,
	) -> Result<ConsensusState, anyhow::Error> {
		use sp_consensus_grandpa::AuthorityList;
		let header = self
			.client
			.rpc()
			.header(Some(hash))
			.await?
			.ok_or_else(|| anyhow!("Header not found for hash: {hash:?}"))?;

		let current_set_id: u64 = {
			let raw_id = self
				.client
				.storage()
				.at(hash)
				.fetch_raw(&GRANDPA_CURRENT_SET_ID[..])
				.await
				.ok()
				.flatten()
				.expect("Failed to fetch current set id");
			codec::Decode::decode(&mut &*raw_id)?
		};

		let current_authorities = {
			let bytes = self
				.client
				.rpc()
				.request::<String>(
					"state_call",
					subxt::rpc_params!(
						"GrandpaApi_grandpa_authorities",
						"0x",
						Some(format!("{:?}", hash))
					),
				)
				.await
				.map(|res| hex::decode(&res[2..]))??;

			AuthorityList::decode(&mut &bytes[..])?
		};

		// Ensure there are no duplicates in authority list
		let mut set = BTreeSet::new();
		for (id, ..) in &current_authorities {
			if !set.insert(id) {
				Err(anyhow!("Duplicate entries found in current authority set"))?
			}
		}

		let latest_height = u32::from(header.number());

		Ok(ConsensusState {
			current_authorities,
			current_set_id,
			latest_height,
			latest_hash: hash.into(),
			slot_duration,
			state_machine: self.options.state_machine,
		})
	}

	/// Returns the grandpa finality proof
	pub async fn query_finality_proof(
		&self,
		previous_finalized_height: u32,
	) -> Result<FinalityProof<DefaultHeader>, anyhow::Error>
	where
		H256: From<T::Hash>,
		u32: From<<T::Header as Header>::Number>,
		<T::Header as Header>::Number: finality_grandpa::BlockNumberOps + One,
	{
		let max_height = previous_finalized_height + self.options.max_block_range;
		let finalized_hash = self.client.rpc().finalized_head().await?;
		let finalized_header = self
			.client
			.rpc()
			.header(Some(finalized_hash))
			.await?
			.ok_or_else(|| anyhow!("Header not found for hash {finalized_hash:#?}"))?;
		let finalized_number = u32::from(finalized_header.number());
		log::trace!(
			"Finalized block number for {}: {finalized_number}",
			self.options.state_machine
		);

		if max_height > finalized_number {
			let encoded = self
				.client
				.rpc()
				.request::<Option<JustificationNotification>>(
					"grandpa_proveFinality",
					rpc_params![finalized_number],
				)
				.await?
				.ok_or_else(|| anyhow!("No justification found for block: {:?}", finalized_number))?
				.0;

			let mut finality_proof = FinalityProof::<DefaultHeader>::decode(&mut &encoded[..])?;
			let justification =
				GrandpaJustification::<T::Header>::decode(&mut &finality_proof.justification[..])?;
			finality_proof.block = justification.commit.target_hash;
			finality_proof.unknown_headers = self
				.query_headers(previous_finalized_height, justification.commit.target_number)
				.await?;

			return Ok(finality_proof);
		}

		let target_block_number = max_height;
		log::trace!(
			"Target block number for {}: {target_block_number}",
			self.options.state_machine
		);

		let mut target_block_hash = self
			.client
			.rpc()
			.block_hash(Some(target_block_number.into()))
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch block has for height {target_block_number}"))?;
		let diff = target_block_number - previous_finalized_height;

		let mut unknown_headers = vec![];
		let pb = ProgressBar::new(diff as u64);
		for height in previous_finalized_height..=target_block_number {
			let hash = self
				.client
				.rpc()
				.block_hash(Some(height.into()))
				.await?
				.ok_or_else(|| anyhow!("Failed to fetch block has for height {height}"))?;
			let header = self
				.client
				.rpc()
				.header(Some(hash))
				.await?
				.ok_or_else(|| anyhow!("Header with hash: {hash:?} not found!"))?;
			let sp_runtime_header = DefaultHeader::decode(&mut header.encode().as_ref())?;
			unknown_headers.push(sp_runtime_header.clone());

			if let Some(_) = find_scheduled_change(&sp_runtime_header) {
				log::trace!(
					"Found set rotation for {} at block number {height:?}",
					self.options.state_machine
				);
				if height != previous_finalized_height {
					target_block_hash = hash;
					// stop here
					break;
				}
			} else {
				// check block justifications
				let grandpa_justification = self
					.client
					.rpc()
					.block(Some(hash))
					.await?
					.ok_or_else(|| anyhow!("Block not found for number: {hash:#?}"))?
					.justifications
					.and_then(|justifications| {
						justifications
							.into_iter()
							.find_map(|(id, proof)| (id == GRANDPA_ENGINE_ID).then_some(proof))
					});
				if let Some(_) = grandpa_justification {
					log::trace!(
						"Found justification for {} at block number {height:?}",
						self.options.state_machine
					);
					target_block_hash = hash;
				}
			}
			pb.inc(1);
		}
		pb.finish_and_clear();

		let block = self
			.client
			.rpc()
			.block(Some(target_block_hash))
			.await?
			.ok_or_else(|| anyhow!("Block not found for number: {:#?}", target_block_hash))?;
		// get GRANDPA justification
		let justification = block
			.justifications
			.and_then(|justifications| {
				justifications
					.into_iter()
					.find_map(|(id, proof)| (id == GRANDPA_ENGINE_ID).then_some(proof))
			})
			.expect("Block {target_block_hash:#?} should contain GRANDPA justification; qed");
		let previously_finalized_hash = self
			.client
			.rpc()
			.block_hash(Some(previous_finalized_height.into()))
			.await?
			.ok_or_else(|| {
				anyhow!(
					"Failed to fetch block for {} at height {previous_finalized_height}",
					self.options.state_machine
				)
			})?;
		let ancestry = AncestryChain::new(&unknown_headers);
		let canonical = ancestry
			.ancestry(previously_finalized_hash.into(), target_block_hash.into())?
			.iter()
			.map(|hash| ancestry.header(hash).cloned())
			.collect::<Option<Vec<_>>>()
			.ok_or_else(|| anyhow!("Invalid ancestry chain"))?;

		// Found valid justification, decode and update finality proof
		let decoded = GrandpaJustification::<T::Header>::decode(&mut &justification[..])?;
		let finality_proof = FinalityProof {
			block: decoded.commit.target_hash,
			justification,
			unknown_headers: canonical,
		};

		Ok(finality_proof)
	}

	/// Query a range of headers from the chain
	///
	/// # Arguments
	///
	/// * `start` - The starting block height
	/// * `end` - The ending block height (inclusive)
	///
	/// # Returns
	///
	/// A vector of decoded headers within the specified range
	///
	/// # Errors
	///
	/// Returns an error if any block hash or header cannot be retrieved
	pub async fn query_headers(
		&self,
		start: u32,
		end: u32,
	) -> Result<Vec<DefaultHeader>, anyhow::Error> {
		let mut headers = Vec::new();
		let pb = ProgressBar::new((start - end) as u64);
		for height in start..=end {
			let hash = self
				.client
				.rpc()
				.block_hash(Some(height.into()))
				.await?
				.ok_or_else(|| anyhow!("Failed to fetch block has for height {height}"))?;
			let header = self
				.client
				.rpc()
				.header(Some(hash))
				.await?
				.ok_or_else(|| anyhow!("Header with hash: {hash:?} not found!"))?;
			headers.push(DefaultHeader::decode(&mut header.encode().as_ref())?);
			pb.inc(1);
		}
		pb.finish_and_clear();
		Ok(headers)
	}

	/// Returns the proof for parachain headers finalized by the provided finality proof
	pub async fn query_finalized_parachain_headers_with_proof(
		&self,
		finalized_hash: T::Hash,
	) -> Result<BTreeMap<H256, ParachainHeaderProofs>, anyhow::Error>
	where
		H256: From<T::Hash>,
		<T::Header as Header>::Number: finality_grandpa::BlockNumberOps + One,
	{
		// we are interested only in the blocks where our parachain header changes.
		let para_keys: Vec<_> = self
			.options
			.para_ids
			.iter()
			.map(|para_id| parachain_header_storage_key(*para_id))
			.collect();
		let keys = para_keys.iter().map(|key| key.as_ref()).collect::<Vec<&[u8]>>();
		let mut parachain_headers_with_proof = BTreeMap::<H256, ParachainHeaderProofs>::default();

		let state_proof = self
			.client
			.rpc()
			.read_proof(keys, Some(finalized_hash))
			.await?
			.proof
			.into_iter()
			.map(|bytes| bytes.0)
			.collect::<Vec<_>>();
		parachain_headers_with_proof.insert(
			finalized_hash.into(),
			ParachainHeaderProofs { state_proof, para_ids: self.options.para_ids.clone() },
		);
		Ok(parachain_headers_with_proof)
	}
}
