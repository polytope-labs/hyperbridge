// Copyright (C) 2022 Polytope Labs.
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

//! BEEFY prover utilities

#![allow(clippy::all)]
#![deny(missing_docs)]

/// Methods for querying the relay chain
pub mod relay;
/// Metadata generated code for interacting with the relay chain
pub mod runtime;
/// Helper functions and types
pub mod util;

/// Some consensus related constants
pub mod constants {

	/// Block at which BEEFY was activated on rococo
	pub const ROCOCO_BEEFY_ACTIVATION_BLOCK: u32 = 3_804_028;
}

use anyhow::anyhow;
use beefy_verifier_primitives::{
	ConsensusMessage, ConsensusState, MmrProof, ParachainHeader, ParachainProof, SignedCommitment,
};
use codec::{Decode, Encode};
use primitive_types::H256;
use relay::{fetch_latest_beefy_justification, fetch_mmr_proof};
use sp_consensus_beefy::{
	ecdsa_crypto::Signature,
	known_payloads::MMR_ROOT_ID,
	mmr::{BeefyNextAuthoritySet, MmrLeaf},
};
use sp_io::hashing::keccak_256;
use sp_mmr_primitives::LeafProof;
use subxt::{rpc_params, Config, OnlineClient};
use util::{hash_authority_addresses, prove_authority_set, AuthorityProofWithSignatures};

/// This contains methods for fetching BEEFY proofs for parachain headers.
#[derive(Clone, Debug)]
pub struct Prover<R: Config, P: Config> {
	/// Height at which beefy was activated.
	pub beefy_activation_block: u32,
	/// Subxt client for the relay chain
	pub relay: OnlineClient<R>,
	/// Subxt client for the parachain
	pub para: OnlineClient<P>,
	/// Para Id for the associated parachains.
	pub para_ids: Vec<u32>,
}

impl<R: Config, P: Config> Prover<R, P> {
	/// Construct a beefy client state to be submitted to the counterparty chain
	pub async fn get_initial_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let latest_finalized_head =
			self.relay.rpc().request("beefy_getFinalizedHead", rpc_params!()).await?;
		let (signed_commitment, latest_beefy_finalized) =
			fetch_latest_beefy_justification(&self.relay, latest_finalized_head).await?;

		// Encoding and decoding to fix dependency version conflicts
		let next_authority_set = {
			let key = runtime::storage().beefy_mmr_leaf().beefy_next_authorities();
			let next_authority_set = self
				.relay
				.storage()
				.at(latest_beefy_finalized)
				.fetch(&key)
				.await?
				.expect("Should retrieve next authority set")
				.encode();
			BeefyNextAuthoritySet::decode(&mut &*next_authority_set)
				.expect("Should decode next authority set correctly")
		};

		let current_authority_set = {
			let key: subxt::storage::Address<
				subxt::utils::Static<subxt::utils::Encoded>,
				runtime::runtime_types::sp_consensus_beefy::mmr::BeefyAuthoritySet<H256>,
				subxt::storage::address::Yes,
				subxt::storage::address::Yes,
				(),
			> = runtime::storage().beefy_mmr_leaf().beefy_authorities();
			let authority_set = self
				.relay
				.storage()
				.at(latest_beefy_finalized)
				.fetch(&key)
				.await?
				.expect("Should retrieve next authority set")
				.encode();
			BeefyNextAuthoritySet::decode(&mut &*authority_set)
				.expect("Should decode next authority set correctly")
		};

		let mmr_root_hash = signed_commitment
			.commitment
			.payload
			.get_decoded::<H256>(&MMR_ROOT_ID)
			.expect("Mmr root hash should decode correctly");

		let client_state = ConsensusState {
			mmr_root_hash,
			beefy_activation_block: self.beefy_activation_block,
			latest_beefy_height: signed_commitment.commitment.block_number as u32,
			current_authorities: current_authority_set.clone(),
			next_authorities: next_authority_set.clone(),
		};

		Ok(client_state)
	}

	/// This will fetch the latest leaf in the mmr as well as a proof for this leaf in the latest
	/// mmr root hash.
	pub async fn consensus_proof(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<u32, Signature>,
	) -> Result<ConsensusMessage, anyhow::Error> {
		let subxt_block_number: subxt::rpc::types::BlockNumber =
			(signed_commitment.commitment.block_number - 1).into();
		let block_hash = self
			.relay
			.rpc()
			.block_hash(Some(subxt_block_number))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		let current_authorities = {
			let key = runtime::storage().beefy().authorities();
			self.relay
				.storage()
				.at(block_hash)
				.fetch(&key)
				.await?
				.ok_or_else(|| anyhow!("No beefy authorities found!"))?
				.0
		};

		// Current LeafIndex
		let block_number = signed_commitment.commitment.block_number;
		let leaf_proof = fetch_mmr_proof(&self.relay, block_number.into()).await?;
		let leaves: Vec<Vec<u8>> = codec::Decode::decode(&mut &*leaf_proof.leaves.0)?;
		let latest_leaf: MmrLeaf<u32, H256, H256, H256> = codec::Decode::decode(&mut &*leaves[0])?;
		let mmr_proof: LeafProof<H256> = Decode::decode(&mut &*leaf_proof.proof.0)?;

		let authority_address_hashes = hash_authority_addresses(
			current_authorities.into_iter().map(|x| x.encode()).collect(),
		)?;

		let AuthorityProofWithSignatures { authority_proof, signatures } =
			prove_authority_set(&signed_commitment, authority_address_hashes)?;

		let mmr = MmrProof {
			signed_commitment: SignedCommitment {
				commitment: signed_commitment.commitment.clone(),
				signatures,
			},
			latest_mmr_leaf: latest_leaf.clone(),
			mmr_proof,
			authority_proof,
		};

		let heads = {
			let key = runtime::storage().paras().parachains();
			let ids = self
				.relay
				.storage()
				.at(block_hash)
				.fetch(&key)
				.await?
				.ok_or_else(|| anyhow!("No beefy authorities found!"))?;

			let mut heads = vec![];
			for id in ids {
				let key = runtime::storage().paras().heads(&id);
				let head = self
					.relay
					.storage()
					.at(block_hash)
					.fetch(&key)
					.await?
					.ok_or_else(|| anyhow!("No beefy authorities found!"))?
					.0;
				heads.push((id.0, head));
			}

			heads.sort();

			heads
		};

		let (parachains, indices): (Vec<_>, Vec<_>) = self
			.para_ids
			.iter()
			.map(|id| {
				let index = heads.iter().position(|(i, _)| *i == *id).expect("ParaId should exist");
				(
					ParachainHeader {
						header: heads[index].1.clone(),
						index,
						para_id: heads[index].0,
					},
					index,
				)
			})
			.unzip();

		let leaves = heads.iter().map(|pair| keccak_256(&pair.encode())).collect::<Vec<_>>();
		let proof = util::merkle_proof(&leaves, &indices);

		let parachain = ParachainProof { parachains, proof };

		Ok(ConsensusMessage { mmr, parachain })
	}
}
