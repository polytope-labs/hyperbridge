// Copyright (C) 2022 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use codec::{Decode, Encode};
use futures::stream::FuturesOrdered;
use hex_literal::hex;
use merkle_mountain_range::{helper::get_peaks, leaf_index_to_mmr_size};
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
use polkadot_sdk::{sp_consensus_beefy::mmr::BeefyAuthoritySet, *};
use primitive_types::H256;
use rs_merkle::MerkleTree;
use sp_consensus_beefy::{
	mmr::{MmrLeaf, MmrLeafVersion},
	SignedCommitment, VersionedFinalityProof,
};
use sp_io::hashing::keccak_256;
use sp_mmr_primitives::LeafProof;
use sp_runtime::{generic::Header, traits::BlakeTwo256};
use sp_storage::StorageKey;
use subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient},
	config::{substrate::SubstrateHeader, HashFor, Header as _},
	ext::subxt_rpcs::rpc_params,
	Config,
};

use crate::{
	util::MerkleHasher, BEEFY_MMR_LEAF_BEEFY_NEXT_AUTHORITIES, BEEFY_VALIDATOR_SET_ID,
	PARAS_PARACHAINS,
};

/// Storage key for mmr.numberOfLeaves
pub const MMR_NUMBER_OF_LEAVES: [u8; 32] =
	hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d");

/// Storage key for mmr.nodes(u64)
pub const MMR_NODES: [u8; 32] =
	hex!("a8c65209d47ee80f56b0011e8fd91f50519dfc7fdad21b84f64a5310fa178ef2");

/// Get beefy justification for block_hash
pub async fn fetch_latest_beefy_justification<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	latest_beefy_finalized: HashFor<T>,
) -> Result<
	(SignedCommitment<u32, sp_consensus_beefy::ecdsa_crypto::Signature>, HashFor<T>),
	anyhow::Error,
> {
	let block = rpc
		.chain_get_block(Some(latest_beefy_finalized))
		.await
		.ok()
		.flatten()
		.expect("Should find a valid block");

	let justifications = block.justifications.expect("Block should have valid justifications");

	let beefy_justification = justifications
		.into_iter()
		.find_map(|justfication| {
			(justfication.0 == sp_consensus_beefy::BEEFY_ENGINE_ID).then(|| justfication.1)
		})
		.expect("Should have valid beefy justification");
	let VersionedFinalityProof::V1(signed_commitment) = VersionedFinalityProof::<
		u32,
		sp_consensus_beefy::ecdsa_crypto::Signature,
	>::decode(&mut &*beefy_justification)
	.expect("Beefy justification should decode correctly");

	Ok((signed_commitment, latest_beefy_finalized))
}

/// Fetch all parachain headers committed by BEEFY at provided height
pub async fn paras_parachains<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	at: Option<HashFor<T>>,
) -> Result<Vec<(u32, Vec<u8>)>, anyhow::Error> {
	let ids = rpc
		.state_get_storage(PARAS_PARACHAINS.as_slice(), at)
		.await?
		.map(|data| Vec::<u32>::decode(&mut data.as_ref()))
		.transpose()?
		.ok_or_else(|| anyhow!("No beefy authorities found!"))?;

	let mut heads = vec![];
	for id in ids {
		let head = rpc
			.state_get_storage(parachain_header_storage_key(id).as_ref(), at)
			.await?
			.map(|data| Vec::<u8>::decode(&mut data.as_ref()))
			.transpose()?
			.ok_or_else(|| anyhow!("No beefy authorities found!"))?;
		heads.push((id, head));
	}
	heads.sort_by_key(|(id, _)| *id);

	Ok(heads)
}

/// Fetch the next BEEFY authority set commitment at the provided height
pub async fn beefy_mmr_leaf_next_authorities<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	at: Option<HashFor<T>>,
) -> Result<BeefyAuthoritySet<H256>, anyhow::Error> {
	// Encoding and decoding to fix dependency version conflicts
	let next_authority_set = {
		let next_authority_set = rpc
			.state_get_storage(BEEFY_MMR_LEAF_BEEFY_NEXT_AUTHORITIES.as_slice(), at)
			.await?
			.expect("Should retrieve next authority set");
		BeefyAuthoritySet::decode(&mut &*next_authority_set)
			.expect("Should decode next authority set correctly")
	};
	Ok(next_authority_set)
}

/// Get beefy justification for latest finalized beefy block
pub async fn fetch_next_beefy_justification<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	rpc_client: RpcClient,
	latest_client_height: u64,
	current_set_id: u64,
) -> Result<
	Option<(SignedCommitment<u32, sp_consensus_beefy::ecdsa_crypto::Signature>, HashFor<T>)>,
	anyhow::Error,
> {
	let mut block_hash = rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await?;

	let (signed_commitment, latest_beefy_finalized) = loop {
		let set_id = rpc
			.state_get_storage(BEEFY_VALIDATOR_SET_ID.as_slice(), Some(block_hash))
			.await?
			.map(|data| u64::decode(&mut data.as_ref()))
			.transpose()?
			.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

		let block = rpc
			.chain_get_block(Some(block_hash))
			.await
			.ok()
			.flatten()
			.expect("Should find a valid block");
		if latest_client_height >= block.block.header.number().into() {
			return Ok(None);
		}

		let justifications = block.justifications;

		let beefy_justification = justifications.and_then(|justifications| {
			justifications.into_iter().find_map(|justfication| {
				(justfication.0 == sp_consensus_beefy::BEEFY_ENGINE_ID).then(|| justfication.1)
			})
		});

		if (current_set_id..=(current_set_id + 1)).contains(&set_id) &&
			beefy_justification.is_some()
		{
			let VersionedFinalityProof::V1(signed_commitment) =
				VersionedFinalityProof::<u32, sp_consensus_beefy::ecdsa_crypto::Signature>::decode(
					&mut &*beefy_justification.unwrap(),
				)
				.expect("Beefy justification should decode correctly");
			break (signed_commitment, block_hash);
		}
		block_hash = SubstrateHeader::<u32, T::Hasher>::decode(&mut &*block.block.header.encode())
			.expect("infallible")
			.parent_hash;
	};

	Ok(Some((signed_commitment, latest_beefy_finalized)))
}

/// Calculates the heights of subtrees in a Merkle Mountain Range (MMR)
/// for a given number of leaves.
///
/// This function decomposes the leaf count into powers of 2 and returns a vector
/// of heights (log base 2 values) that represent the height of each subtree in the MMR.
///
/// # Arguments
/// * `leaves_length` - The total number of leaves in the MMR
///
/// # Returns
/// A vector of u32 values representing the heights of each subtree
fn subtree_heights(leaves_length: u64) -> Vec<u64> {
	let max_subtrees = 1024;
	let mut indices = vec![];
	let mut i = 0;
	let mut current = leaves_length;

	while i < max_subtrees {
		if current == 0 {
			break;
		}

		let log = current.ilog2();
		indices.push(log as u64);
		current = current - u64::pow(2, log);

		i += 1;
	}

	indices
}

/// Queries the MMR leaf at a specific block hash.
///
/// This function retrieves the MMR (Merkle Mountain Range) leaf for a given block hash
/// by first fetching the block header and then constructing the leaf with parent hash,
/// next authority set, and parachain data.
///
/// # Arguments
/// * `client` - The online client to interact with the blockchain
/// * `block_hash` - The hash of the block to query
///
/// # Returns
/// * `Result<MmrLeaf<u32, H256, H256, H256>, anyhow::Error>` - The MMR leaf on success, or an error
pub async fn query_mmr_leaf<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	block_hash: HashFor<T>,
) -> Result<MmrLeaf<u32, H256, H256, H256>, anyhow::Error> {
	let header = rpc
		.chain_get_header(Some(block_hash))
		.await?
		.ok_or_else(|| anyhow!("Block hash not found"))?;

	let header = Header::<u32, BlakeTwo256>::decode(&mut &header.encode()[..])?;
	let parent_hash = HashFor::<T>::decode(&mut header.parent_hash.as_ref())?;
	let beefy_next_authority_set = beefy_mmr_leaf_next_authorities(rpc, Some(block_hash)).await?;
	let leaf_extra = {
		let heads = paras_parachains(rpc, Some(parent_hash)).await?;

		// Calculate leaf hashes from the parachain headers
		let leaf_hashes = heads.iter().map(|leaf| keccak_256(&leaf.encode())).collect::<Vec<_>>();

		let tree = MerkleTree::<MerkleHasher>::from_leaves(&leaf_hashes);
		let root = tree
			.root()
			.ok_or_else(|| anyhow!("Failed to parachain heads calculate root!"))?;
		H256(root)
	};
	let leaf = MmrLeaf {
		version: MmrLeafVersion::new(0, 0),
		parent_number_and_hash: (header.number - 1, header.parent_hash),
		beefy_next_authority_set,
		leaf_extra,
	};

	Ok(leaf)
}

/// Query the mmr proof for the leaf at the provided block number
pub async fn fetch_mmr_proof<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	block_number: u32,
	query_batch_size: Option<u32>,
) -> Result<(LeafProof<H256>, MmrLeaf<u32, H256, H256, H256>), anyhow::Error> {
	use tokio_stream::StreamExt;
	let block_hash = rpc
		.chain_get_block_hash(Some(block_number.into()))
		.await?
		.ok_or_else(|| anyhow!("Block hash not found for block {block_number}"))?;

	let number_of_mmr_leaves = {
		let encoded = rpc
			.state_get_storage(MMR_NUMBER_OF_LEAVES.as_slice(), Some(block_hash))
			.await?
			.expect("Should retrieve total number of leaves in mmr");

		u64::decode(&mut &encoded[..])?
	};
	let subtrees = subtree_heights(number_of_mmr_leaves);
	let last_subtree_height =
		subtrees.last().ok_or_else(|| anyhow!("Invalid number of leaves"))?.clone() as u32;

	let leaf_index = number_of_mmr_leaves - 1;
	let peaks = get_peaks(leaf_index_to_mmr_size(leaf_index));
	debug_assert!(subtrees.len() == peaks.len());

	let mut proof = vec![];

	// add all peak roots to proof with exception of last peak
	for peak in peaks.iter().take(peaks.len().saturating_sub(1)) {
		let key = [MMR_NODES.to_vec(), u64::encode(peak)].concat();

		let encoded = rpc
			.state_get_storage(&key, Some(block_hash))
			.await?
			.expect(&format!("Should retrieve hash for peak {peak}"));

		let peak_root = H256::decode(&mut &encoded[..])?;
		proof.push(peak_root);
	}

	let pb = indicatif::ProgressBar::with_draw_target(
		Some(2u32.pow(last_subtree_height) as u64),
		indicatif::ProgressDrawTarget::stdout(),
	);
	let leaf = if last_subtree_height != 0 {
		// construct the proof for the last peak manually
		let subtree_leaves = u32::pow(2, last_subtree_height as u32);
		let first_leaf = block_number - subtree_leaves + 1;

		let mut leaves = vec![];
		let range = (first_leaf..=block_number).into_iter().collect::<Vec<_>>();
		// get all leaves in the peak
		for chunk in range.chunks(query_batch_size.unwrap_or(200) as usize) {
			let processes = chunk
				.into_iter()
				.map(|block| {
					let rpc = rpc.clone();
					let block = *block;
					tokio::spawn(async move {
						// we try to reconstruct the mmr leaf from onchain data because offchain db
						// where mmr_generateProof fetches leaves from might be corrupted
						let block_hash = rpc
							.chain_get_block_hash(Some(block.into()))
							.await?
							.ok_or_else(|| anyhow!("Block hash not found"))?;

						let leaf = query_mmr_leaf(&rpc, block_hash).await?;
						Ok::<_, anyhow::Error>(leaf)
					})
				})
				.collect::<FuturesOrdered<_>>();

			let leaf_batch = processes
				.collect::<Result<Vec<_>, _>>()
				.await?
				.into_iter()
				.collect::<Result<Vec<_>, _>>()?;
			leaves.extend(leaf_batch);

			pb.inc(query_batch_size.unwrap_or(200).into());
		}

		pb.finish_with_message("Finished downloading leaves");

		// manually generate the merkle proof for the last peak
		let leaf_hashes = leaves.iter().map(|leaf| keccak_256(&leaf.encode())).collect::<Vec<_>>();
		let tree = MerkleTree::<MerkleHasher>::from_leaves(&leaf_hashes);
		let items = tree.proof(&[leaves.len() - 1]);
		proof.extend(items.proof_hashes().into_iter().map(|item| H256::from_slice(item)));

		leaves.pop().expect("leaves is always > 1; qed")
	} else {
		query_mmr_leaf(rpc, block_hash).await?
	};

	Ok((
		LeafProof {
			items: proof,
			leaf_count: number_of_mmr_leaves,
			leaf_indices: vec![number_of_mmr_leaves - 1],
		},
		leaf,
	))
}

/// This returns the storage key under which the parachain header with a given para_id is stored.
pub fn parachain_header_storage_key(para_id: u32) -> StorageKey {
	let mut storage_key = frame_support::storage::storage_prefix(b"Paras", b"Heads").to_vec();
	let encoded_para_id = para_id.encode();
	storage_key.extend_from_slice(sp_io::hashing::twox_64(&encoded_para_id).as_slice());
	storage_key.extend_from_slice(&encoded_para_id);
	StorageKey(storage_key)
}

#[cfg(test)]
mod tests {
	use codec::{Decode, Encode};
	use hex_literal::hex;
	use merkle_mountain_range::{Merge, MerkleProof};
	use polkadot_sdk::{
		sp_io::hashing::keccak_256,
		sp_mmr_primitives::mmr_lib::{leaf_index_to_mmr_size, leaf_index_to_pos},
	};
	use primitive_types::H256;
	use subxt::{
		backend::{legacy::LegacyRpcMethods, rpc::RpcClient},
		PolkadotConfig,
	};

	use crate::relay::{fetch_mmr_proof, subtree_heights};

	fn get_peak_pos_by_height(height: u32) -> u64 {
		(1 << (height + 1)) - 2
	}

	struct MmrMerge;

	impl Merge for MmrMerge {
		type Item = H256;
		fn merge(
			lhs: &Self::Item,
			rhs: &Self::Item,
		) -> Result<Self::Item, merkle_mountain_range::Error> {
			let mut concat = vec![];
			concat.extend(&lhs.0);
			concat.extend(&rhs.0);
			Ok(H256(keccak_256(&concat)))
		}
	}

	/// Storage key for mmr.rootHash()
	pub const MMR_ROOT_HASH: [u8; 32] =
		hex!("a8c65209d47ee80f56b0011e8fd91f50d42f676807518c67bb427546ba406fa1");

	#[tokio::test]
	async fn test_mmr_proof() {
		let Ok(ws_url) = std::env::var("RELAY_WS_URL") else { return };
		let _relay = subxt_utils::client::ws_client::<PolkadotConfig>(&ws_url, u32::MAX)
			.await
			.unwrap();
		let relay_rpc_client = RpcClient::from_url(&ws_url).await.unwrap();
		let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());

		for block in 25420896..25420999 {
			dbg!();
			dbg!(block);
			let (proof, leaf) = fetch_mmr_proof(&relay_rpc, block, None).await.unwrap();

			// dbg!(&leaf);

			let mmr_size = leaf_index_to_mmr_size(proof.leaf_indices[0]);
			let mmr_proof = MerkleProof::<_, MmrMerge>::new(mmr_size, proof.items.clone());
			let root = mmr_proof
				.calculate_root(vec![(
					leaf_index_to_pos(proof.leaf_indices[0]),
					H256(keccak_256(&leaf.encode())),
				)])
				.unwrap();

			let block_hash =
				relay_rpc.chain_get_block_hash(Some(block.into())).await.unwrap().unwrap();
			let onchain_root = {
				let encoded = relay_rpc
					.state_get_storage(&MMR_ROOT_HASH, Some(block_hash))
					.await
					.unwrap()
					.expect("Should retrieve mmr root hash");

				H256::decode(&mut &encoded[..]).unwrap()
			};
			dbg!(root == onchain_root);
			dbg!();

			assert_eq!(root, onchain_root);
		}
	}

	#[test]
	fn test_subtree_heights() {
		// pos_height_in_tree(pos);
		dbg!(subtree_heights(5869895));
		dbg!(subtree_heights(5869895)
			.into_iter()
			.map(|height| get_peak_pos_by_height(height as u32))
			.collect::<Vec<_>>());
		dbg!(subtree_heights(5921822));
	}
}
