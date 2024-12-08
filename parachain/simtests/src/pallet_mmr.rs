#![cfg(test)]

use std::{env, time::Duration};

use codec::Decode;
use merkle_mountain_range::MerkleProof;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, keccak_256, offchain::StorageKind, Bytes, H256};
use sp_keyring::sr25519::Keyring;
use sp_mmr_primitives::{mmr_lib::leaf_index_to_pos, utils::NodesUtils, INDEXING_PREFIX};
use sp_runtime::traits::Keccak256;
use subxt::{rpc_params, tx::SubmittableExtrinsic, utils::H160};

use merkle_mountain_range::util::MemMMR;
use mmr_primitives::{DataOrHash, FullLeaf};
use pallet_ismp::offchain::{Leaf, ProofKeys};
use pallet_mmr::mmr::Hasher as MmrHasher;
use subxt_utils::{
	gargantua, gargantua::api::runtime_types::pallet_ismp_demo::pallet::EvmParams, Hyperbridge,
};

#[tokio::test]
#[ignore]
async fn test_all_features() -> Result<(), anyhow::Error> {
	dispatch_requests().await?;
	Ok(())
}

#[cfg(feature = "stress-test")]
#[tokio::test]
async fn test_insert_1_billion_mmr_leaves() -> Result<(), anyhow::Error> {
	// try to estimate the storage requirements on offchaindb with 1 billion leaves in mmr
	use indicatif::ProgressBar;

	let port = env::var("PORT").unwrap_or("9990".into());
	let client = subxt_utils::client::ws_client::<Hyperbridge>(
		&format!("ws://127.0.0.1:{}", port),
		u32::MAX,
	)
	.await?;
	let pb = ProgressBar::new(100_000);
	for pos in 44_243..100_000 {
		// Initialize MMR Pallet by dispatching some leaves and finalizing
		let params =
			EvmParams { module: H160::random(), destination: 1, timeout: 0, count: 10_000 };
		let call = client
			.tx()
			.call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
		let _account_id = AsRef::<[u8; 32]>::as_ref(&Keyring::Ferdie.to_account_id()).clone();

		let _ = client
			.rpc()
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		let extrinsic: Bytes = client
			.rpc()
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice
				rpc_params![Bytes::from(call), Keyring::Ferdie.to_account_id().to_ss58check()],
			)
			.await?;
		SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0).submit().await?;

		let created_block = client
			.rpc()
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		// Finalize a new block so that we are sure mmr gadget gets the notification
		let _ = client
			.rpc()
			.request::<bool>("engine_finalizeBlock", rpc_params![created_block.hash])
			.await?;
		pb.set_position(pos);
	}

	pb.finish_with_message("Inserted 1 billion leaves");

	Ok(())
}

async fn dispatch_requests() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let client = subxt_utils::client::ws_client::<Hyperbridge>(
		&format!("ws://127.0.0.1:{}", port),
		u32::MAX,
	)
	.await?;

	let address = subxt_utils::gargantua::api::storage().mmr().number_of_leaves();
	let leaf_count_at_start = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address)
		.await
		.unwrap()
		.unwrap_or_default();
	dbg!(leaf_count_at_start);
	let get_child_trie_root = |block_hash: H256| {
		let client = client.clone();
		let block_hash = block_hash.clone();
		let address = subxt_utils::gargantua::api::storage().ismp().child_trie_root();
		async move { client.storage().at(block_hash).fetch(&address).await.unwrap() }
	};

	// Initialize MMR Pallet by dispatching some leaves and finalizing
	let params = EvmParams { module: H160::random(), destination: 1, timeout: 0, count: 10 };
	let call = client
		.tx()
		.call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
	let extrinsic: Bytes = client
		.rpc()
		.request(
			"simnode_authorExtrinsic",
			// author an extrinsic from alice
			rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
	submittable.submit().await?;
	let created_block = client
		.rpc()
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	let mut last_finalized = created_block.hash;
	let _ = client
		.rpc()
		.request::<bool>("engine_finalizeBlock", rpc_params![last_finalized])
		.await?;

	for _ in 0..3 {
		let created_block = client
			.rpc()
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		last_finalized = created_block.hash;
	}

	// Finalize a new block so that we are sure mmr gadget gets the notification
	let _ = client
		.rpc()
		.request::<bool>("engine_finalizeBlock", rpc_params![last_finalized])
		.await?;

	// Wait for some seconds for the async mmr gadget to complete
	tokio::time::sleep(Duration::from_secs(30)).await;

	// Get finalized leaves
	let mut leaves = vec![];
	for idx in 0..(leaf_count_at_start + 10) {
		let pos = leaf_index_to_pos(idx as u64);
		let canon_key = NodesUtils::node_canon_offchain_key(INDEXING_PREFIX, pos);
		let value = client
			.rpc()
			.request::<Option<Bytes>>(
				"offchain_localStorageGet",
				rpc_params![StorageKind::PERSISTENT, Bytes::from(canon_key)],
			)
			.await?;
		assert!(value.is_some());
		let data = value.unwrap().0;
		let leaf = DataOrHash::<Keccak256, Leaf>::decode(&mut &*data).unwrap();
		leaves.push(leaf);
	}

	// Dispatch some requests on two chain forks

	let mut chain_a = vec![];
	let mut chain_b = vec![];

	let mut chain_a_commitments = vec![];
	let mut chain_b_commitments = vec![];

	// Fork A
	{
		let mut parent_hash = last_finalized;
		for _ in 0..3 {
			let params =
				EvmParams { module: H160::random(), destination: 1, timeout: 0, count: 10 };
			let call = client
				.tx()
				.call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
			let extrinsic: Bytes = client
				.rpc()
				.request(
					"simnode_authorExtrinsic",
					// author an extrinsic from alice
					rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
				)
				.await?;
			let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
			submittable.submit().await?;
			let created_block = client
				.rpc()
				.request::<CreatedBlock<H256>>(
					"engine_createBlock",
					rpc_params![true, false, Some(parent_hash)],
				)
				.await?;
			let events = client.events().at(created_block.hash).await?;

			let child_trie_root = get_child_trie_root(created_block.hash).await.unwrap();
			dbg!(child_trie_root);
			let events = events
				.iter()
				.filter_map(|ev| {
					ev.ok().and_then(|ev| {
						ev.as_event::<gargantua::api::ismp::events::Request>()
							.ok()
							.flatten()
							.and_then(|ev| Some((child_trie_root, ev.commitment)))
					})
				})
				.collect::<Vec<_>>();

			chain_a_commitments.extend(events);
			parent_hash = created_block.hash;
			chain_a.push(created_block.hash);
		}
	}

	println!("Finished creating Fork A");

	println!("Creating Fork B");

	// Fork B
	{
		let mut parent_hash = last_finalized;
		let accounts = vec![
			Keyring::Bob.to_account_id().to_ss58check(),
			Keyring::Eve.to_account_id().to_ss58check(),
			Keyring::Dave.to_account_id().to_ss58check(),
		];
		for i in 0..accounts.len() {
			let params =
				EvmParams { module: H160::random(), destination: 97, timeout: 0, count: 10 };
			let call = client
				.tx()
				.call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
			let extrinsic: Bytes = client
				.rpc()
				.request(
					"simnode_authorExtrinsic",
					rpc_params![Bytes::from(call), accounts[i].clone()],
				)
				.await?;
			let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
			submittable.submit().await?;
			let created_block = client
				.rpc()
				.request::<CreatedBlock<H256>>(
					"engine_createBlock",
					rpc_params![true, false, parent_hash],
				)
				.await?;

			let events = client.events().at(created_block.hash).await?;

			let child_trie_root = get_child_trie_root(created_block.hash).await.unwrap();
			dbg!(child_trie_root);
			let events = events
				.iter()
				.filter_map(|ev| {
					ev.ok().and_then(|ev| {
						ev.as_event::<gargantua::api::ismp::events::Request>()
							.ok()
							.flatten()
							.and_then(|ev| Some((child_trie_root, ev.commitment)))
					})
				})
				.collect::<Vec<_>>();

			chain_b_commitments.extend(events);
			chain_b.push(created_block.hash);
			parent_hash = created_block.hash;
		}
	}

	assert_eq!(chain_a_commitments.len(), 30);
	assert_eq!(chain_b_commitments.len(), 30);

	println!("Finished creating Fork B");

	// Fetch mmr leaves on Fork A pre-finality

	let initial_leaf_count = leaves.len() as u64;
	dbg!(initial_leaf_count);
	let mut positions = vec![];
	for (idx, (prefix, _)) in chain_a_commitments.clone().into_iter().enumerate() {
		let pos = leaf_index_to_pos(initial_leaf_count + idx as u64);
		positions.push(pos);
		let non_canon_key = NodesUtils::node_temp_offchain_key::<
			sp_runtime::generic::Header<u32, Keccak256>,
		>(INDEXING_PREFIX, pos, prefix);
		let value = client
			.rpc()
			.request::<Option<Bytes>>(
				"offchain_localStorageGet",
				rpc_params![StorageKind::PERSISTENT, Bytes::from(non_canon_key)],
			)
			.await?;
		assert!(value.is_some());
		let data = value.unwrap().0;
		let leaf = DataOrHash::<Keccak256, Leaf>::decode(&mut &*data).unwrap();
		leaves.push(leaf);
	}

	dbg!(positions.len());

	// Finalize fork a
	let res = client
		.rpc()
		.request::<bool>("engine_finalizeBlock", rpc_params![chain_a.last().cloned().unwrap()])
		.await?;
	assert!(res);

	let finalized_block = client
		.rpc()
		.request::<CreatedBlock<H256>>(
			"engine_createBlock",
			rpc_params![true, false, chain_a.last().cloned().unwrap()],
		)
		.await?;

	// Finalize again so stale branches can be pruned
	let _ = client
		.rpc()
		.request::<bool>("engine_finalizeBlock", rpc_params![finalized_block.hash])
		.await?;

	// Wait for some time for the async worker to complete
	tokio::time::sleep(Duration::from_secs(20)).await;

	// All Non canonical keys should no longer exist in storage as they should have been pruned
	for (idx, (prefix, _)) in chain_b_commitments.into_iter().enumerate() {
		let pos = leaf_index_to_pos(initial_leaf_count + idx as u64);
		let non_canon_key = NodesUtils::node_temp_offchain_key::<
			sp_runtime::generic::Header<u32, Keccak256>,
		>(INDEXING_PREFIX, pos, prefix);
		let value = client
			.rpc()
			.request::<Option<Bytes>>(
				"offchain_localStorageGet",
				rpc_params![StorageKind::PERSISTENT, Bytes::from(non_canon_key)],
			)
			.await?;
		assert!(value.is_none());
	}

	// Canonical keys should exist and the commitment should match the commitments we have for chain
	// B
	for (idx, (prefix, commitment)) in chain_a_commitments.clone().into_iter().enumerate() {
		let pos = leaf_index_to_pos(initial_leaf_count + idx as u64);
		let non_canon_key = NodesUtils::node_temp_offchain_key::<
			sp_runtime::generic::Header<u32, Keccak256>,
		>(INDEXING_PREFIX, pos, prefix);
		let canon_key = NodesUtils::node_canon_offchain_key(INDEXING_PREFIX, pos);
		let value = client
			.rpc()
			.request::<Option<Bytes>>(
				"offchain_localStorageGet",
				rpc_params![StorageKind::PERSISTENT, Bytes::from(non_canon_key)],
			)
			.await?;
		assert!(value.is_none());

		let value = client
			.rpc()
			.request::<Option<Bytes>>(
				"offchain_localStorageGet",
				rpc_params![StorageKind::PERSISTENT, Bytes::from(canon_key)],
			)
			.await?;

		let data = value.unwrap().0;
		let leaf = match DataOrHash::<Keccak256, Leaf>::decode(&mut &*data).unwrap() {
			DataOrHash::Data(leaf) => leaf,
			_ => unreachable!(),
		};
		let request = keccak_256(&leaf.preimage());
		assert_eq!(commitment.0, request);
	}

	let finalized_hash = chain_a.last().cloned().unwrap();
	let address = subxt_utils::gargantua::api::storage().mmr().number_of_leaves();
	let mmr_leaf_count =
		client.storage().at(finalized_hash).fetch(&address).await.unwrap().unwrap();
	assert_eq!(mmr_leaf_count, leaves.len() as u64);
	// Construct mmr tree from pre-finalized leaves
	let mut mmr = MemMMR::<DataOrHash<Keccak256, Leaf>, MmrHasher<Keccak256, Leaf>>::default();
	for leaf in leaves.clone() {
		mmr.push(leaf).unwrap();
	}

	let at = client.rpc().header(Some(finalized_hash)).await?.unwrap().number;

	// Fetch mmr proof from finalized branch
	let keys = ProofKeys::Requests(
		chain_a_commitments.into_iter().map(|(.., commitment)| commitment).collect(),
	);
	let params = rpc_params![at, keys];
	let response: pallet_ismp_rpc::Proof =
		client.rpc().request("ismp_queryMmrProof", params).await?;
	let proof: pallet_ismp::offchain::Proof<H256> = Decode::decode(&mut &*response.proof)?;

	let merkle_proof = MerkleProof::<DataOrHash<Keccak256, Leaf>, MmrHasher<Keccak256, Leaf>>::new(
		mmr.mmr_size(),
		proof.items.into_iter().map(DataOrHash::Hash).collect(),
	);

	let root = mmr.get_root().unwrap();
	let res = merkle_proof
		.verify(
			root,
			positions
				.into_iter()
				.zip(leaves[(initial_leaf_count as usize)..].to_vec())
				.collect(),
		)
		.unwrap();

	assert!(res);
	Ok(())
}
