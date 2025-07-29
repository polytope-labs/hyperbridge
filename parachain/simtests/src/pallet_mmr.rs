#![cfg(test)]

use std::{env, time::Duration};

use codec::{Decode, Encode};
use hex_literal::hex;
use merkle_mountain_range::MerkleProof;
use pallet_ismp_demo::EvmParams;
use polkadot_sdk::*;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, keccak_256, offchain::StorageKind, Bytes, H256};
use sp_keyring::sr25519::Keyring;
use sp_mmr_primitives::{mmr_lib::leaf_index_to_pos, utils::NodesUtils, INDEXING_PREFIX};
use sp_runtime::traits::Keccak256;
use subxt::{
	backend::legacy::LegacyRpcMethods, ext::subxt_rpcs::rpc_params, tx::SubmittableTransaction,
	utils::H160,
};

use merkle_mountain_range::util::MemMMR;
use mmr_primitives::DataOrHash;
use pallet_ismp::offchain::{FullLeaf, Leaf, ProofKeys};
use pallet_mmr_tree::mmr::Hasher as MmrHasher;
use subxt::ext::{scale_decode::DecodeAsType, scale_encode::EncodeAsType};
use subxt_utils::{values::evm_params_to_value, Hyperbridge};

const NUMBER_OF_LEAVES_KEY: [u8; 32] =
	hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d");
const CHILD_TRIE_ROOT_KEY: [u8; 32] =
	hex!("103895530afb23bb607661426d55eb8b2a9069bca219dffc57c97e59931e2396");

/// Currently supported state machines.
#[derive(
	Clone,
	Debug,
	Copy,
	Encode,
	Decode,
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Hash,
	DecodeAsType,
	EncodeAsType,
)]
#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
pub enum StateMachine {
	/// Evm state machines
	#[codec(index = 0)]
	Evm(u32),
	/// Polkadot parachains
	#[codec(index = 1)]
	Polkadot(u32),
	/// Kusama parachains
	#[codec(index = 2)]
	Kusama(u32),
}

#[derive(Decode, Encode, DecodeAsType, EncodeAsType, Clone, Debug, Eq, PartialEq)]
#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
pub struct RequestEvent {
	/// Chain that this request will be routed to
	dest_chain: StateMachine,
	/// Source Chain for request
	source_chain: StateMachine,
	/// Request nonce
	request_nonce: u64,
	/// Commitment
	commitment: subxt::utils::H256,
}

impl subxt::events::StaticEvent for RequestEvent {
	const PALLET: &'static str = "Ismp";
	const EVENT: &'static str = "Request";
}

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
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;
	let rpc = LegacyRpcMethods::<Hyperbridge>::new(rpc_client.clone());

	let pb = ProgressBar::new(100_000);
	for pos in 44_243..100_000 {
		// Initialize MMR Pallet by dispatching some leaves and finalizing
		let params =
			EvmParams { module: H160::random(), destination: 1, timeout: 0, count: 10_000 };
		let call = client
			.tx()
			.call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
		let _account_id = AsRef::<[u8; 32]>::as_ref(&Keyring::Ferdie.to_account_id()).clone();

		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice
				rpc_params![Bytes::from(call), Keyring::Ferdie.to_account_id().to_ss58check()],
			)
			.await?;
		SubmittableTransaction::from_bytes(client.clone(), extrinsic.0).submit().await?;

		let created_block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		// Finalize a new block so that we are sure mmr gadget gets the notification
		let _ = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![created_block.hash])
			.await?;
		pb.set_position(pos);
	}

	pb.finish_with_message("Inserted 1 billion leaves");

	Ok(())
}

async fn dispatch_requests() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;
	let rpc = LegacyRpcMethods::<Hyperbridge>::new(rpc_client.clone());

	let leaf_count_at_start = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(&NUMBER_OF_LEAVES_KEY)
		.await
		.unwrap()
		.unwrap_or_default();
	let leaf_count_at_start: u64 =
		Decode::decode(&mut &*leaf_count_at_start).ok().unwrap_or_default();
	dbg!(leaf_count_at_start);
	let get_child_trie_root = |block_hash: H256| {
		let client = client.clone();
		let block_hash = block_hash.clone();
		async move {
			let raw = client
				.storage()
				.at(block_hash)
				.fetch_raw(&CHILD_TRIE_ROOT_KEY)
				.await
				.ok()
				.flatten()
				.unwrap();
			H256::decode(&mut &*raw)
		}
	};

	// Initialize MMR Pallet by dispatching some leaves and finalizing
	let params = EvmParams { module: H160::random(), destination: 1, timeout: 0, count: 10 };
	let call =
		subxt::dynamic::tx("IsmpDemo", "dispatch_to_evm", vec![evm_params_to_value(&params)]);

	let call = client.tx().call_data(&call)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			// author an extrinsic from alice
			rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	submittable.submit().await?;
	let created_block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	let mut last_finalized = created_block.hash;
	let _ = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![last_finalized])
		.await?;

	for _ in 0..3 {
		let created_block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		last_finalized = created_block.hash;
	}

	// Finalize a new block so that we are sure mmr gadget gets the notification
	let _ = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![last_finalized])
		.await?;

	// Wait for some seconds for the async mmr gadget to complete
	tokio::time::sleep(Duration::from_secs(30)).await;

	// Get finalized leaves
	let mut leaves = vec![];
	for idx in 0..(leaf_count_at_start + 10) {
		let pos = leaf_index_to_pos(idx as u64);
		let canon_key = NodesUtils::node_canon_offchain_key(INDEXING_PREFIX, pos);
		let value = rpc_client
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
			let call = subxt::dynamic::tx(
				"IsmpDemo",
				"dispatch_to_evm",
				vec![evm_params_to_value(&params)],
			);

			let call = client.tx().call_data(&call)?;
			let extrinsic: Bytes = rpc_client
				.request(
					"simnode_authorExtrinsic",
					// author an extrinsic from alice
					rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
				)
				.await?;
			let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
			submittable.submit().await?;
			let created_block = rpc_client
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
						ev.as_event::<RequestEvent>()
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
			let call = subxt::dynamic::tx(
				"IsmpDemo",
				"dispatch_to_evm",
				vec![evm_params_to_value(&params)],
			);

			let call = client.tx().call_data(&call)?;
			let extrinsic: Bytes = rpc_client
				.request(
					"simnode_authorExtrinsic",
					rpc_params![Bytes::from(call), accounts[i].clone()],
				)
				.await?;
			let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
			submittable.submit().await?;
			let created_block = rpc_client
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
						ev.as_event::<RequestEvent>()
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
		let value = rpc_client
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
	let res = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![chain_a.last().cloned().unwrap()])
		.await?;
	assert!(res);

	let finalized_block = rpc_client
		.request::<CreatedBlock<H256>>(
			"engine_createBlock",
			rpc_params![true, false, chain_a.last().cloned().unwrap()],
		)
		.await?;

	// Finalize again so stale branches can be pruned
	let _ = rpc_client
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
		let value = rpc_client
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
		let value = rpc_client
			.request::<Option<Bytes>>(
				"offchain_localStorageGet",
				rpc_params![StorageKind::PERSISTENT, Bytes::from(non_canon_key)],
			)
			.await?;
		assert!(value.is_none());

		let value = rpc_client
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
	let mmr_leaf_count = client
		.storage()
		.at(finalized_hash)
		.fetch_raw(&NUMBER_OF_LEAVES_KEY)
		.await
		.unwrap()
		.unwrap_or_default();
	let mmr_leaf_count: u64 = Decode::decode(&mut &*mmr_leaf_count).unwrap();
	assert_eq!(mmr_leaf_count, leaves.len() as u64);
	// Construct mmr tree from pre-finalized leaves
	let mut mmr = MemMMR::<DataOrHash<Keccak256, Leaf>, MmrHasher<Keccak256, Leaf>>::default();
	for leaf in leaves.clone() {
		mmr.push(leaf).unwrap();
	}

	let at = rpc.chain_get_header(Some(finalized_hash)).await?.unwrap().number;

	// Fetch mmr proof from finalized branch
	let keys = ProofKeys::Requests(
		chain_a_commitments
			.into_iter()
			.map(|(.., commitment)| commitment.0.into())
			.collect(),
	);
	let params = rpc_params![at, keys];
	let response: pallet_ismp_rpc::Proof = rpc_client.request("mmr_queryProof", params).await?;
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
