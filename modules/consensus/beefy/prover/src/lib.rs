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

/// Re-export of `rs_merkle` so downstream crates use the same version as the prover
/// (necessary because `MerkleHasher` only implements `Hasher` for this version).
pub use rs_merkle;

use anyhow::anyhow;
use codec::{Decode, Encode};
use hex_literal::hex;
use polkadot_sdk::*;
use primitive_types::H256;
use sp_consensus_beefy::{
	ecdsa_crypto::Signature, known_payloads::MMR_ROOT_ID, mmr::BeefyAuthoritySet,
};
use sp_io::hashing::keccak_256;
use subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient},
	ext::subxt_rpcs::rpc_params,
	Config, OnlineClient,
};
use subxt_core::config::HashFor;

use beefy_verifier_primitives::{
	ConsensusMessage, ConsensusState, MmrProof, ParachainHeader, ParachainProof,
	SignatureWithAuthorityIndex, SignedCommitment,
};
use fiat_shamir::{
	compute_commitment_hash, derive_authority_challenge, filter_signatures_for_challenge,
	SignersBitmap,
};
use relay::{
	beefy_mmr_leaf_next_authorities, fetch_latest_beefy_justification, fetch_mmr_proof,
	paras_parachains,
};
use util::hash_authority_addresses;

/// Fiat-Shamir transcript for deterministic validator sampling
pub mod fiat_shamir;
/// Methods for querying the relay chain
pub mod relay;
/// Helper functions and types
pub mod util;

/// This contains methods for fetching BEEFY proofs for parachain headers.
#[derive(Clone, Debug)]
pub struct Prover<R: Config, P: Config> {
	/// Height at which beefy was activated.
	pub beefy_activation_block: u32,
	/// Subxt client for the relay chain
	pub relay: OnlineClient<R>,
	/// Rpc for the relay chain
	pub relay_rpc: LegacyRpcMethods<R>,
	/// Rpc client for making rpc request for the relay chain
	pub relay_rpc_client: RpcClient,
	/// Subxt client for the parachain
	pub para: OnlineClient<P>,
	/// Rpc for the parachain
	pub para_rpc: LegacyRpcMethods<P>,
	/// Rpc client for making rpc request for the parachain
	pub para_rpc_client: RpcClient,
	/// Para Id for the associated parachains.
	pub para_ids: Vec<u32>,
	/// Leaf chunk size
	pub query_batch_size: Option<u32>,
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
	pub async fn get_initial_consensus_state(
		&self,
		at: Option<HashFor<R>>,
	) -> Result<ConsensusState, anyhow::Error> {
		let latest_finalized_head = if let Some(at) = at {
			at
		} else {
			self.relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await?
		};
		let (signed_commitment, latest_beefy_finalized) =
			fetch_latest_beefy_justification(&self.relay_rpc, latest_finalized_head).await?;

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
			next_authorities: beefy_mmr_leaf_next_authorities(
				&self.relay_rpc,
				Some(latest_beefy_finalized),
			)
			.await?,
		};

		Ok(client_state)
	}

	/// Fetch the current BEEFY authority set commitment at the provided height
	pub async fn mmr_leaf_current_authorities(
		&self,
		at: Option<HashFor<R>>,
	) -> Result<BeefyAuthoritySet<H256>, anyhow::Error> {
		let current_authority_set = {
			let authority_set = self
				.relay_rpc
				.state_get_storage(BEEFY_MMR_LEAF_BEEFY_AUTHORITIES.as_slice(), at)
				.await?
				.expect("Should retrieve next authority set");
			BeefyAuthoritySet::decode(&mut &*authority_set)?
		};

		Ok(current_authority_set)
	}

	/// Fetch the BEEFY authority public keys at the provided height
	pub async fn beefy_authorities(
		&self,
		at: Option<HashFor<R>>,
	) -> Result<Vec<[u8; 33]>, anyhow::Error> {
		// Encoding and decoding to fix dependency version conflicts
		let current_authorities = {
			self.relay_rpc
				.state_get_storage(BEEFY_AUTHORITIES.as_slice(), at)
				.await?
				.map(|data| Vec::<[u8; 33]>::decode(&mut data.as_ref()))
				.transpose()?
				.ok_or_else(|| anyhow!("No beefy authorities found!"))?
		};
		Ok(current_authorities)
	}

	/// Produces a Fiat-Shamir consensus proof that only includes [`fiat_shamir::SAMPLE_SIZE`]
	/// validator signatures, selected deterministically via a Fiat-Shamir transcript.
	///
	/// The prover computes the same transcript as the on-chain `BeefyV1FiatShamir` verifier
	/// to learn which authority indices will be challenged, then includes only those
	/// signatures and their merkle membership proofs.
	///
	/// # Arguments
	///
	/// * `signed_commitment` — The full BEEFY signed commitment from the relay chain.
	/// * `consensus_state` — The current consensus state, used to determine the active authority
	///   set (root + length) for transcript construction.
	pub async fn consensus_proof_fiat_shamir(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<u32, Signature>,
		consensus_state: &ConsensusState,
	) -> Result<(ConsensusMessage, SignersBitmap), anyhow::Error> {
		let block_number: u32 = signed_commitment.commitment.block_number.into();
		let block_hash = self
			.relay_rpc
			.chain_get_block_hash(Some(block_number.into()))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		let (mmr_proof, latest_leaf) =
			fetch_mmr_proof(&self.relay_rpc, block_number.try_into()?, self.query_batch_size)
				.await?;

		// Determine the active authority set based on the validator_set_id in the commitment
		let authority_set = if signed_commitment.commitment.validator_set_id ==
			consensus_state.next_authorities.id
		{
			&consensus_state.next_authorities
		} else {
			&consensus_state.current_authorities
		};

		// Build the signers bitmap from the signed commitment
		let bitmap = SignersBitmap::from_signed_commitment(&signed_commitment);
		let signer_count = bitmap.count_set_bits(authority_set.len);

		// Derive the Fiat-Shamir challenged authority indices — identical to the
		// on-chain verifier's `deriveAuthorityChallenge`.
		let commitment_hash = compute_commitment_hash(&signed_commitment.commitment);
		let challenged_indices = derive_authority_challenge(
			commitment_hash,
			authority_set.keyset_commitment,
			authority_set.len,
			&bitmap,
			signer_count,
		);

		// Build the merkle proof for exactly those challenged authorities
		let current_authorities = self.beefy_authorities(Some(block_hash)).await?;
		let authority_count = current_authorities.len();

		// Extract and process only the challenged signatures
		let mut signatures = filter_signatures_for_challenge(
			&signed_commitment,
			&challenged_indices,
			authority_count,
		)?;

		let authority_address_hashes = hash_authority_addresses(
			current_authorities.into_iter().map(|x| x.encode()).collect(),
		)?;
		let indices = signatures.iter().map(|x| x.index as usize).collect::<Vec<_>>();

		// Build the merkle proof and convert it to positioned format using the
		// helper documented in the solidity-merkle-trees README.
		let tree =
			rs_merkle::MerkleTree::<util::MerkleHasher>::from_leaves(&authority_address_hashes);
		let proof = tree.proof(&indices);
		let leaf_hashes: Vec<[u8; 32]> =
			indices.iter().map(|&i| authority_address_hashes[i]).collect();
		let (proof_nodes, leaf_nodes) =
			util::convert_proof(&proof, &indices, &leaf_hashes, authority_address_hashes.len());

		// Each signature's leaf_position comes from the leaf_nodes returned by convert_proof.
		// leaf_nodes is sorted by position, but signatures are in their original order, so
		// we look each one up by its index.
		for sig in &mut signatures {
			let leaf = leaf_nodes
				.iter()
				.find(|n| {
					let first_leaf_pos = 1usize <<
						rs_merkle::utils::indices::tree_depth(authority_address_hashes.len());
					n.position == first_leaf_pos + sig.index as usize
				})
				.expect("leaf for signature must exist");
			sig.leaf_position = leaf.position as u32;
		}

		let authority_proof: Vec<(usize, [u8; 32])> =
			proof_nodes.into_iter().map(|n| (n.position, n.hash)).collect();

		let mmr = MmrProof {
			signed_commitment: SignedCommitment {
				commitment: signed_commitment.commitment.clone(),
				signatures,
			},
			latest_mmr_leaf: latest_leaf.clone(),
			mmr_proof,
			authority_proof,
		};

		// Build parachain proofs (identical to naive consensus_proof)
		let heads = paras_parachains(
			&self.relay_rpc,
			Some(HashFor::<R>::decode(&mut &*latest_leaf.parent_number_and_hash.1.encode())?),
		)
		.await?;

		let leaves = heads.iter().map(|pair| keccak_256(&pair.encode())).collect::<Vec<_>>();
		let leaf_count = leaves.len();

		let para_indices: Vec<usize> = self
			.para_ids
			.iter()
			.map(|id| heads.iter().position(|(i, _)| *i == *id).expect("ParaId should exist"))
			.collect();

		let para_tree = rs_merkle::MerkleTree::<util::MerkleHasher>::from_leaves(&leaves);
		let para_proof = para_tree.proof(&para_indices);
		let para_leaf_hashes: Vec<[u8; 32]> = para_indices.iter().map(|&i| leaves[i]).collect();
		let (proof_nodes, leaf_nodes) =
			util::convert_proof(&para_proof, &para_indices, &para_leaf_hashes, leaf_count);

		let first_leaf_pos = 1usize << rs_merkle::utils::indices::tree_depth(leaf_count);
		let parachains: Vec<_> = leaf_nodes
			.iter()
			.map(|leaf| {
				let index = leaf.position - first_leaf_pos;
				ParachainHeader {
					header: heads[index].1.clone(),
					index: index as u32,
					leaf_position: leaf.position as u32,
					para_id: heads[index].0,
				}
			})
			.collect();

		let proof: Vec<(u32, [u8; 32])> =
			proof_nodes.into_iter().map(|n| (n.position as u32, n.hash)).collect();

		let parachain = ParachainProof { parachains, proof, total_leaves: leaf_count as u32 };

		Ok((ConsensusMessage { mmr, parachain }, bitmap))
	}

	/// This will fetch the latest leaf in the mmr as well as a proof for this leaf in the latest
	/// mmr root hash.
	pub async fn consensus_proof(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<u32, Signature>,
	) -> Result<ConsensusMessage, anyhow::Error> {
		let block_number: u32 = signed_commitment.commitment.block_number.into();
		let block_hash = self
			.relay_rpc
			.chain_get_block_hash(Some(block_number.into()))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		let (mmr_proof, latest_leaf) =
			fetch_mmr_proof(&self.relay_rpc, block_number.try_into()?, self.query_batch_size)
				.await?;

		// create authorities proof
		let current_authorities = self.beefy_authorities(Some(block_hash)).await?;
		let mut signatures = signed_commitment
			.signatures
			.iter()
			.enumerate()
			.filter_map(|(index, x)| {
				let sig = x.as_ref()?;
				if sig.len() != 65 {
					return None;
				}
				let mut temp = [0u8; 65];
				temp.copy_from_slice(&*sig.encode());
				let last = temp.last_mut().unwrap();
				*last = *last + 27;
				Some(SignatureWithAuthorityIndex {
					index: index as u32,
					// filled in below once we know the leaf positions from convert_proof
					leaf_position: 0,
					signature: temp,
				})
			})
			.collect::<Vec<_>>();

		let authority_address_hashes = hash_authority_addresses(
			current_authorities.into_iter().map(|x| x.encode()).collect(),
		)?;
		let indices = signatures.iter().map(|x| x.index as usize).collect::<Vec<_>>();

		// Build the merkle proof and convert it to positioned format using the
		// helper documented in the solidity-merkle-trees README.
		let tree =
			rs_merkle::MerkleTree::<util::MerkleHasher>::from_leaves(&authority_address_hashes);
		let proof = tree.proof(&indices);
		let leaf_hashes: Vec<[u8; 32]> =
			indices.iter().map(|&i| authority_address_hashes[i]).collect();
		let (proof_nodes, leaf_nodes) =
			util::convert_proof(&proof, &indices, &leaf_hashes, authority_address_hashes.len());

		// Each signature's leaf_position comes from leaf_nodes (sorted by position).
		let first_leaf_pos =
			1usize << rs_merkle::utils::indices::tree_depth(authority_address_hashes.len());
		for sig in &mut signatures {
			let target = first_leaf_pos + sig.index as usize;
			sig.leaf_position = leaf_nodes
				.iter()
				.find(|n| n.position == target)
				.expect("leaf for signature must exist")
				.position as u32;
		}

		let authority_proof: Vec<(usize, [u8; 32])> =
			proof_nodes.into_iter().map(|n| (n.position, n.hash)).collect();

		let mmr = MmrProof {
			signed_commitment: SignedCommitment {
				commitment: signed_commitment.commitment.clone(),
				signatures,
			},
			latest_mmr_leaf: latest_leaf.clone(),
			mmr_proof,
			authority_proof,
		};

		let heads = paras_parachains(
			&self.relay_rpc,
			Some(HashFor::<R>::decode(&mut &*latest_leaf.parent_number_and_hash.1.encode())?),
		)
		.await?;

		let leaves = heads.iter().map(|pair| keccak_256(&pair.encode())).collect::<Vec<_>>();
		let leaf_count = leaves.len();

		let indices: Vec<usize> = self
			.para_ids
			.iter()
			.map(|id| heads.iter().position(|(i, _)| *i == *id).expect("ParaId should exist"))
			.collect();

		let para_tree = rs_merkle::MerkleTree::<util::MerkleHasher>::from_leaves(&leaves);
		let para_proof = para_tree.proof(&indices);
		let para_leaf_hashes: Vec<[u8; 32]> = indices.iter().map(|&i| leaves[i]).collect();
		let (proof_nodes, leaf_nodes) =
			util::convert_proof(&para_proof, &indices, &para_leaf_hashes, leaf_count);

		let first_leaf_pos = 1usize << rs_merkle::utils::indices::tree_depth(leaf_count);
		let parachains = leaf_nodes
			.iter()
			.map(|leaf| {
				let index = leaf.position - first_leaf_pos;
				ParachainHeader {
					header: heads[index].1.clone(),
					index: index as u32,
					leaf_position: leaf.position as u32,
					para_id: heads[index].0,
				}
			})
			.collect();

		let proof: Vec<(u32, [u8; 32])> =
			proof_nodes.into_iter().map(|n| (n.position as u32, n.hash)).collect();

		let parachain = ParachainProof { parachains, proof, total_leaves: leaf_count as u32 };

		Ok(ConsensusMessage { mmr, parachain })
	}
}
