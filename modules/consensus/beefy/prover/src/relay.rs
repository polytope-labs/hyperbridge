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

use anyhow::anyhow;
use codec::{Decode, Encode};
use mmr_rpc::LeavesProof;
use primitive_types::H256;
use sp_consensus_beefy::{SignedCommitment, VersionedFinalityProof};
use sp_storage::StorageKey;
use std::collections::BTreeMap;
use subxt::{
	config::{substrate::SubstrateHeader, Header},
	rpc::rpc_params,
	Config, OnlineClient,
};

use crate::BEEFY_VALIDATOR_SET_ID;

/// This contains the leaf indices of the relay chain blocks and a map of relay chain heights to a
/// map of all parachain headers at those heights Used for generating [`ParaHeadsProof`]
pub struct FinalizedParaHeads {
	/// Block numbers
	pub block_numbers: Vec<u32>,
	/// Map of relay chain heights to map of para ids and parachain headers SCALE-encoded
	pub raw_finalized_heads: BTreeMap<u64, BTreeMap<u32, Vec<u8>>>,
}

/// Get beefy justification for block_hash
pub async fn fetch_latest_beefy_justification<T: Config>(
	client: &OnlineClient<T>,
	latest_beefy_finalized: T::Hash,
) -> Result<
	(SignedCommitment<u32, sp_consensus_beefy::ecdsa_crypto::Signature>, T::Hash),
	anyhow::Error,
> {
	let block = client
		.rpc()
		.block(Some(latest_beefy_finalized))
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

/// Get beefy justification for latest finalized beefy block
pub async fn fetch_next_beefy_justification<T: Config>(
	client: &OnlineClient<T>,
	latest_client_height: u64,
	current_set_id: u64,
) -> Result<
	Option<(SignedCommitment<u32, sp_consensus_beefy::ecdsa_crypto::Signature>, T::Hash)>,
	anyhow::Error,
> {
	let mut block_hash = client.rpc().request("beefy_getFinalizedHead", rpc_params!()).await?;

	let (signed_commitment, latest_beefy_finalized) = loop {
		let set_id = client
			.rpc()
			.storage(BEEFY_VALIDATOR_SET_ID.as_slice(), Some(block_hash))
			.await?
			.map(|data| u64::decode(&mut data.as_ref()))
			.transpose()?
			.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

		let block = client
			.rpc()
			.block(Some(block_hash))
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

/// Query a mmr  proof
pub async fn fetch_mmr_proof<T: Config>(
	client: &OnlineClient<T>,
	block_number: u32,
) -> Result<LeavesProof<H256>, anyhow::Error> {
	let block_hash = client.rpc().block_hash(Some(block_number.into())).await?;

	let proof: LeavesProof<H256> = client
		.rpc()
		.request(
			"mmr_generateProof",
			rpc_params!(vec![block_number], Option::<T::Hash>::None, block_hash),
		)
		.await?;
	Ok(proof)
}

/// This returns the storage key under which the parachain header with a given para_id is stored.
pub fn parachain_header_storage_key(para_id: u32) -> StorageKey {
	let mut storage_key = frame_support::storage::storage_prefix(b"Paras", b"Heads").to_vec();
	let encoded_para_id = para_id.encode();
	storage_key.extend_from_slice(sp_io::hashing::twox_64(&encoded_para_id).as_slice());
	storage_key.extend_from_slice(&encoded_para_id);
	StorageKey(storage_key)
}
