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
/// Helper functions and types
pub mod util;
use polkadot_sdk::*;

use anyhow::anyhow;
use beefy_verifier_primitives::{
	ConsensusMessage, ConsensusState, MmrProof, ParachainHeader, ParachainProof,
	SignatureWithAuthorityIndex, SignedCommitment,
};
use codec::{Decode, Encode};
use hex_literal::hex;
use primitive_types::H256;
use relay::{fetch_latest_beefy_justification, fetch_mmr_proof, parachain_header_storage_key};
use sp_consensus_beefy::{
	ecdsa_crypto::Signature,
	known_payloads::MMR_ROOT_ID,
	mmr::{BeefyAuthoritySet, MmrLeaf},
};
use sp_io::hashing::keccak_256;
use sp_mmr_primitives::LeafProof;
use subxt::{rpc_params, Config, OnlineClient};
use util::hash_authority_addresses;

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

#[cfg(not(feature = "local"))]
/// Relay chain storage key for beefMmrLeaf.beefyNextAuthorites()
pub const BEEFY_MMR_LEAF_BEEFY_NEXT_AUTHORITIES: [u8; 32] =
	hex!("2ecf93be7260df120a495bd3855c0e600c98535b82c72faf3c64974094af4643");
#[cfg(not(feature = "local"))]
/// Relay chain storage key for beefMmrLeaf.beefyAuthorites()
pub const BEEFY_MMR_LEAF_BEEFY_AUTHORITIES: [u8; 32] =
	hex!("2ecf93be7260df120a495bd3855c0e60c52aa943bf0908860a3eea0fad707cdc");
#[cfg(feature = "local")]
/// Relay chain storage key for beefMmrLeaf.beefyNextAuthorites()
pub const BEEFY_MMR_LEAF_BEEFY_NEXT_AUTHORITIES: [u8; 32] =
	hex!("da7d4185f8093e80caceb64da45219e30c98535b82c72faf3c64974094af4643");
#[cfg(feature = "local")]
/// Relay chain storage key for beefMmrLeaf.beefyAuthorites()
pub const BEEFY_MMR_LEAF_BEEFY_AUTHORITIES: [u8; 32] =
	hex!("da7d4185f8093e80caceb64da45219e3c52aa943bf0908860a3eea0fad707cdc");
/// Relay chain storage key for beefy.authorities()
pub const BEEFY_AUTHORITIES: [u8; 32] =
	hex!("08c41974a97dbf15cfbec28365bea2da5e0621c4869aa60c02be9adcc98a0d1d");
/// Relay chain storage key for beefy.validatorSetId()
pub const BEEFY_VALIDATOR_SET_ID: [u8; 32] =
	hex!("08c41974a97dbf15cfbec28365bea2da8f05bccc2f70ec66a32999c5761156be");
/// Relay chain storage key for paras.parachains()
pub const PARAS_PARACHAINS: [u8; 32] =
	hex!("cd710b30bd2eab0352ddcc26417aa1940b76934f4cc08dee01012d059e1b83ee");

impl<R: Config, P: Config> Prover<R, P> {
	/// Construct a beefy client state to be submitted to the counterparty chain
	pub async fn get_initial_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let latest_finalized_head =
			self.relay.rpc().request("beefy_getFinalizedHead", rpc_params!()).await?;
		let (signed_commitment, latest_beefy_finalized) =
			fetch_latest_beefy_justification(&self.relay, latest_finalized_head).await?;

		let mmr_root_hash = signed_commitment
			.commitment
			.payload
			.get_decoded::<H256>(&MMR_ROOT_ID)
			.expect("Mmr root hash should decode correctly");

		let client_state = ConsensusState {
			mmr_root_hash,
			beefy_activation_block: self.beefy_activation_block,
			latest_beefy_height: signed_commitment.commitment.block_number as u32,
			current_authorities: self
				.mmr_leaf_current_authorities(Some(latest_beefy_finalized))
				.await?,
			next_authorities: self.mmr_leaf_next_authorities(Some(latest_beefy_finalized)).await?,
		};

		Ok(client_state)
	}

	/// Fetch the current BEEFY authority set commitment at the provided height
	pub async fn mmr_leaf_current_authorities(
		&self,
		at: Option<R::Hash>,
	) -> Result<BeefyAuthoritySet<H256>, anyhow::Error> {
		let current_authority_set = {
			let authority_set = self
				.relay
				.rpc()
				.storage(BEEFY_MMR_LEAF_BEEFY_AUTHORITIES.as_slice(), at)
				.await?
				.expect("Should retrieve next authority set")
				.0;
			BeefyAuthoritySet::decode(&mut &*authority_set)?
		};

		Ok(current_authority_set)
	}

	/// Fetch the next BEEFY authority set commitment at the provided height
	pub async fn mmr_leaf_next_authorities(
		&self,
		at: Option<R::Hash>,
	) -> Result<BeefyAuthoritySet<H256>, anyhow::Error> {
		// Encoding and decoding to fix dependency version conflicts
		let next_authority_set = {
			let next_authority_set = self
				.relay
				.rpc()
				.storage(BEEFY_MMR_LEAF_BEEFY_NEXT_AUTHORITIES.as_slice(), at)
				.await?
				.expect("Should retrieve next authority set")
				.0;
			BeefyAuthoritySet::decode(&mut &*next_authority_set)
				.expect("Should decode next authority set correctly")
		};
		Ok(next_authority_set)
	}

	/// Fetch the BEEFY authority public keys at the provided height
	pub async fn beefy_authorities(
		&self,
		at: Option<R::Hash>,
	) -> Result<Vec<[u8; 33]>, anyhow::Error> {
		// Encoding and decoding to fix dependency version conflicts
		let current_authorities = {
			self.relay
				.rpc()
				.storage(BEEFY_AUTHORITIES.as_slice(), at)
				.await?
				.map(|data| Vec::<[u8; 33]>::decode(&mut data.as_ref()))
				.transpose()?
				.ok_or_else(|| anyhow!("No beefy authorities found!"))?
		};
		Ok(current_authorities)
	}

	/// Fetch all parachain headers committed by BEEFY at provided height
	pub async fn paras_parachains(
		&self,
		at: Option<R::Hash>,
	) -> Result<Vec<(u32, Vec<u8>)>, anyhow::Error> {
		let ids = self
			.relay
			.rpc()
			.storage(PARAS_PARACHAINS.as_slice(), at)
			.await?
			.map(|data| Vec::<u32>::decode(&mut data.as_ref()))
			.transpose()?
			.ok_or_else(|| anyhow!("No beefy authorities found!"))?;

		let mut heads = vec![];
		for id in ids {
			let head = self
				.relay
				.rpc()
				.storage(parachain_header_storage_key(id).as_ref(), at)
				.await?
				.map(|data| Vec::<u8>::decode(&mut data.as_ref()))
				.transpose()?
				.ok_or_else(|| anyhow!("No beefy authorities found!"))?;
			heads.push((id, head));
		}
		heads.sort();

		Ok(heads)
	}

	/// This will fetch the latest leaf in the mmr as well as a proof for this leaf in the latest
	/// mmr root hash.
	pub async fn consensus_proof(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<u32, Signature>,
	) -> Result<ConsensusMessage, anyhow::Error> {
		let block_number: subxt::rpc::types::NumberOrHex =
			signed_commitment.commitment.block_number.into();
		let block_hash = self
			.relay
			.rpc()
			.block_hash(Some(block_number.into()))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		// Current LeafIndex
		let leaf_proof = fetch_mmr_proof(&self.relay, block_number.try_into()?).await?;
		let leaves: Vec<Vec<u8>> = codec::Decode::decode(&mut &*leaf_proof.leaves.0)?;
		let latest_leaf: MmrLeaf<u32, H256, H256, H256> = codec::Decode::decode(&mut &*leaves[0])?;
		let mmr_proof: LeafProof<H256> = Decode::decode(&mut &*leaf_proof.proof.0)?;

		// create authorities proof
		let signatures = signed_commitment
			.signatures
			.iter()
			.enumerate()
			.map(|(index, x)| {
				if let Some(sig) = x {
					let mut temp = [0u8; 65];
					if sig.len() == 65 {
						temp.copy_from_slice(&*sig.encode());
						let last = temp.last_mut().unwrap();
						*last = *last + 27;
						Some(SignatureWithAuthorityIndex { index: index as u32, signature: temp })
					} else {
						None
					}
				} else {
					None
				}
			})
			.filter_map(|x| x)
			.collect::<Vec<_>>();
		let current_authorities = self.beefy_authorities(Some(block_hash)).await?;
		let authority_address_hashes = hash_authority_addresses(
			current_authorities.into_iter().map(|x| x.encode()).collect(),
		)?;
		let indices = signatures.iter().map(|x| x.index as usize).collect::<Vec<_>>();
		let authority_proof = util::merkle_proof(&authority_address_hashes, &indices);

		let mmr = MmrProof {
			signed_commitment: SignedCommitment {
				commitment: signed_commitment.commitment.clone(),
				signatures,
			},
			latest_mmr_leaf: latest_leaf.clone(),
			mmr_proof,
			authority_proof,
		};

		let heads = self
			.paras_parachains(Some(R::Hash::decode(
				&mut &*latest_leaf.parent_number_and_hash.1.encode(),
			)?))
			.await?;
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
