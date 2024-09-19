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

//! GRANDPA prover utilities

use anyhow::anyhow;
use codec::{Decode, Encode};
use grandpa_verifier_primitives::{
	parachain_header_storage_key, ConsensusState, FinalityProof, ParachainHeaderProofs,
	ParachainHeadersWithFinalityProof,
};
use ismp::host::StateMachine;
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
	/// ParaId of the associated parachains
	pub para_ids: Vec<u32>,
	/// State machine identifier for the chain
	pub state_machine: StateMachine,
	/// Storage for babe epoch start
	pub babe_epoch_start: Vec<u8>,
	/// Storage key for current set id
	pub current_set_id: Vec<u8>,
}

// We redefine these here because we want the header to be bounded by subxt::config::Header in the
// prover
/// Commit
pub type Commit = finality_grandpa::Commit<H256, u32, AuthoritySignature, AuthorityId>;

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
	/// Initializes the parachain and relay chain clients given the ws urls.
	pub async fn new(
		ws_url: &str,
		para_ids: Vec<u32>,
		state_machine: StateMachine,
		babe_epoch_start: Vec<u8>,
		current_set_id: Vec<u8>,
	) -> Result<Self, anyhow::Error> {
		let max_rpc_payload_size = 15 * 1024 * 1024;
		let client = subxt_utils::client::ws_client(ws_url, max_rpc_payload_size).await?;

		Ok(Self { client, para_ids, state_machine, babe_epoch_start, current_set_id })
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
				.fetch_raw(&self.current_set_id[..])
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
			current_set_id: current_set_id + 1,
			latest_height,
			latest_hash: hash.into(),
			slot_duration,
			state_machine: self.state_machine,
		})
	}

	/// Returns the grandpa finality proof
	pub async fn query_finality_proof<H>(
		&self,
		previous_finalized_height: u32,
		mut latest_finalized_height: u32,
	) -> Result<FinalityProof<H>, anyhow::Error>
	where
		H: Header + codec::Decode,
		u32: From<<H as Header>::Number>,
		<H::Hasher as subxt::config::Hasher>::Output: From<T::Hash>,
		T::Hash: From<<H::Hasher as subxt::config::Hasher>::Output>,
		H::Number: finality_grandpa::BlockNumberOps + One,
	{
		let encoded = self
			.client
			.rpc()
			.request::<Option<JustificationNotification>>(
				"grandpa_proveFinality",
				rpc_params![latest_finalized_height],
			)
			.await?
			.ok_or_else(|| {
				anyhow!("No justification found for block: {:?}", latest_finalized_height)
			})?
			.0;

		let mut finality_proof = FinalityProof::<H>::decode(&mut &encoded[..])?;

		let justification =
			GrandpaJustification::<H>::decode(&mut &finality_proof.justification[..])?;

		finality_proof.block = justification.commit.target_hash;

		latest_finalized_height = u32::from(justification.commit.target_number);

		let mut unknown_headers = vec![];
		for height in previous_finalized_height..=latest_finalized_height {
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

			unknown_headers.push(H::decode(&mut &header.encode()[..])?);
		}

		// overwrite unknown headers
		finality_proof.unknown_headers = unknown_headers;
		Ok(finality_proof)
	}

	/// Returns the proof for parachain headers finalized by the provided finality proof
	pub async fn query_finalized_parachain_headers_with_proof<H>(
		&self,
		_previous_finalized_height: u32,
		latest_finalized_height: u32,
		finality_proof: FinalityProof<H>,
	) -> Result<ParachainHeadersWithFinalityProof<H>, anyhow::Error>
	where
		H: Header + codec::Decode,
		u32: From<<H as Header>::Number>,
		<H::Hasher as subxt::config::Hasher>::Output: From<T::Hash>,
		T::Hash: From<<H::Hasher as subxt::config::Hasher>::Output>,
		H::Number: finality_grandpa::BlockNumberOps + One,
	{
		// we are interested only in the blocks where our parachain header changes.
		let para_keys: Vec<_> = self
			.para_ids
			.iter()
			.map(|para_id| parachain_header_storage_key(*para_id))
			.collect();
		let keys = para_keys.iter().map(|key| key.as_ref()).collect::<Vec<&[u8]>>();
		let mut parachain_headers_with_proof = BTreeMap::<H256, ParachainHeaderProofs>::default();

		let latest_finalized_hash = self
			.client
			.rpc()
			.block_hash(Some(latest_finalized_height.into()))
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch previous finalized hash + 1"))?;

		let state_proof = self
			.client
			.rpc()
			.read_proof(keys, Some(latest_finalized_hash))
			.await?
			.proof
			.into_iter()
			.map(|bytes| bytes.0)
			.collect::<Vec<_>>();
		parachain_headers_with_proof.insert(
			latest_finalized_hash.into(),
			ParachainHeaderProofs { state_proof, para_ids: self.para_ids.clone() },
		);
		Ok(ParachainHeadersWithFinalityProof {
			finality_proof,
			parachain_headers: parachain_headers_with_proof,
		})
	}

	/// Queries the block at which the epoch for the given block belongs to ends.
	pub async fn session_start_and_end_for_block(
		&self,
		block: u32,
	) -> Result<(u32, u32), anyhow::Error> {
		let block_hash = self
			.client
			.rpc()
			.block_hash(Some(block.into()))
			.await?
			.ok_or(anyhow!("Failed to fetch block hash"))?;
		let bytes = self
			.client
			.storage()
			.at(block_hash)
			.fetch_raw(&self.babe_epoch_start[..])
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch epoch information"))?;

		let (previous_epoch_start, current_epoch_start): (u32, u32) =
			codec::Decode::decode(&mut &*bytes)?;
		Ok((
			current_epoch_start,
			current_epoch_start + (current_epoch_start - previous_epoch_start),
		))
	}

	/// Returns the session length in blocks
	pub async fn session_length(&self) -> Result<u32, anyhow::Error> {
		let metadata = self.client.rpc().metadata().await?;
		let metadata = metadata
			.pallet_by_name_err("Babe")?
			.constant_by_name("EpochDuration")
			.ok_or(anyhow!("Failed to fetch constant"))?;
		Ok(Decode::decode(&mut metadata.value())?)
	}
}
