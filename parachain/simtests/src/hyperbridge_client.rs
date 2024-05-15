#![cfg(test)]

use crate::pallet_ismp::Keccak256;
use anyhow::anyhow;
use codec::Encode;
use ismp::{
	host::StateMachine,
	messaging::hash_request,
	router::{Post, Request},
};
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes, KeccakHasher};
use sp_keyring::sr25519::Keyring;
use sp_trie::{LayoutV0, MemoryDB};
use std::{
	collections::HashSet,
	env,
	time::{SystemTime, UNIX_EPOCH},
};
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
use subxt::{error::RpcError, rpc_params, tx::SubmittableExtrinsic};
use subxt_utils::{
	gargantua::{
		api,
		api::{
			runtime_types,
			runtime_types::{
				gargantua_runtime::RuntimeCall,
				ismp::{
					consensus::{StateCommitment, StateMachineHeight, StateMachineId},
					messaging::{Message, Proof, RequestMessage},
				},
			},
		},
	},
	Hyperbridge,
};
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

#[tokio::test]
#[ignore]
async fn test_will_accept_paid_requests() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let client = subxt_utils::client::ws_client::<Hyperbridge>(
		&format!("ws://127.0.0.1:{}", port),
		u32::MAX,
	)
	.await?;

	let unit = 1_000_000_000_000u128;
	let per_byte_fee = 10 * unit;
	let para_id = 3000u32;
	// 1. initialize the ismp parachain client by adding the whitelisted paraId
	{
		let calls = vec![
			RuntimeCall::IsmpParachain(
				runtime_types::ismp_parachain::pallet::Call::add_parachain {
					para_ids: vec![para_id],
				},
			),
			// init the host executive
			RuntimeCall::HostExecutive(
				runtime_types::pallet_ismp_host_executive::pallet::Call::set_host_params {
					params: vec![
						(
							StateMachine::Polkadot(para_id).into(),
							runtime_types::pallet_ismp_host_executive::params::HostParam::SubstrateHostParam(
								runtime_types::pallet_hyperbridge::VersionedHostParams::V1(per_byte_fee)
							)
						)
					],
				},
			),
		];
		let call =
			RuntimeCall::Utility(runtime_types::pallet_utility::pallet::Call::batch_all { calls });
		let call = client.tx().call_data(&api::tx().sudo().sudo(call))?;
		let extrinsic: Bytes = client
			.rpc()
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await?;
		let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = client
			.rpc()
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		let finalized = client
			.rpc()
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	let post = Post {
		source: StateMachine::Polkadot(para_id),
		dest: StateMachine::Polygon,
		nonce: 0,
		from: H256::random().as_bytes().to_vec(),
		to: H256::random().as_bytes().to_vec(),
		timeout_timestamp: 0,
		data: H256::random().as_bytes().to_vec(),
	};
	let request = Request::Post(post.clone());

	let commitment = hash_request::<Keccak256>(&request);
	let mut db = <MemoryDB<KeccakHasher>>::default();
	let mut root = Default::default();
	let mut trie = TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
	let commitment_key = pallet_ismp::child_trie::request_commitment_storage_key(commitment);
	let payment_key = pallet_hyperbridge::child_trie::request_payment_storage_key(commitment);
	let commitment_value = H256::random().as_bytes().to_vec();
	let payment_value = (post.data.len() as u128 * per_byte_fee).encode();

	trie.insert(&commitment_key, &commitment_value).unwrap();
	trie.insert(&payment_key, &payment_value).unwrap();
	drop(trie);

	let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
		.with_recorder(&mut recorder)
		.build();

	assert_eq!(trie.get(&commitment_key).unwrap().unwrap(), commitment_value);
	assert_eq!(trie.get(&payment_key).unwrap().unwrap(), payment_value);

	let proof = recorder
		.drain()
		.into_iter()
		.map(|f| f.data)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	// 2. first set the state commitment using sudo set_storage
	let state_commitment =
		StateCommitment { timestamp: 0, overlay_root: Some(root), state_root: root };
	let height = StateMachineHeight {
		id: StateMachineId {
			state_id: StateMachine::Polkadot(para_id).into(),
			consensus_state_id: *b"PARA",
		},
		height: 200,
	};
	let address1 = api::storage().ismp().state_commitments(&height);
	let address2 = api::storage().ismp().state_machine_update_time(&height);
	let key1 = client.storage().address_bytes(&address1)?;
	let key2 = client.storage().address_bytes(&address2)?;
	let start = SystemTime::now();
	let now = start.duration_since(UNIX_EPOCH).expect("Time went backwards");

	let call = RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_storage {
		items: vec![(key1, state_commitment.encode()), (key2, now.as_secs().encode())],
	});
	let call = client.tx().call_data(&api::tx().sudo().sudo(call))?;
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

	// create a block
	let _ = client
		.rpc()
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	// sanity check that it was properly stored
	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address1)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(item, state_commitment);

	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address2)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(item, now.as_secs());

	let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
		hasher: HashAlgorithm::Keccak,
		storage_proof: proof,
	})
	.encode();
	let proof = Proof { height, proof };

	// 3. next send the requests
	let tx = api::tx().ismp().handle_unsigned(vec![Message::Request(RequestMessage {
		requests: vec![post.clone().into()],
		proof: proof.clone(),
		signer: H256::random().as_bytes().to_vec(),
	})]);

	let progress = client.tx().create_unsigned(&tx)?.submit_and_watch().await?;
	let block = client
		.rpc()
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	let finalized = client
		.rpc()
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;

	Ok(())
}

#[tokio::test]
#[ignore]
async fn test_will_reject_unpaid_requests() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let client = subxt_utils::client::ws_client::<Hyperbridge>(
		&format!("ws://127.0.0.1:{}", port),
		u32::MAX,
	)
	.await?;

	let unit = 1_000_000_000_000u128;
	let per_byte_fee = 10 * unit;
	let para_id = 3000u32;
	// 1. initialize the ismp parachain client by adding the whitelisted paraId
	{
		let calls = vec![
			RuntimeCall::IsmpParachain(
				runtime_types::ismp_parachain::pallet::Call::add_parachain {
					para_ids: vec![para_id],
				},
			),
			// init the host executive
			RuntimeCall::HostExecutive(
				runtime_types::pallet_ismp_host_executive::pallet::Call::set_host_params {
					params: vec![
						(
							StateMachine::Polkadot(para_id).into(),
							runtime_types::pallet_ismp_host_executive::params::HostParam::SubstrateHostParam(
								runtime_types::pallet_hyperbridge::VersionedHostParams::V1(per_byte_fee)
							)
						)
					],
				},
			),
		];
		let call =
			RuntimeCall::Utility(runtime_types::pallet_utility::pallet::Call::batch_all { calls });
		let call = client.tx().call_data(&api::tx().sudo().sudo(call))?;
		let extrinsic: Bytes = client
			.rpc()
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await?;
		let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = client
			.rpc()
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		let finalized = client
			.rpc()
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	let post = Post {
		source: StateMachine::Polkadot(para_id),
		dest: StateMachine::Polygon,
		nonce: 0,
		from: H256::random().as_bytes().to_vec(),
		to: H256::random().as_bytes().to_vec(),
		timeout_timestamp: 0,
		data: H256::random().as_bytes().to_vec(),
	};
	let request = Request::Post(post.clone());

	let commitment = hash_request::<Keccak256>(&request);
	let mut db = <MemoryDB<KeccakHasher>>::default();
	let mut root = Default::default();
	let mut trie = TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
	let commitment_key = pallet_ismp::child_trie::request_commitment_storage_key(commitment);
	let commitment_value = H256::random().as_bytes().to_vec();
	trie.insert(&commitment_key, &commitment_value).unwrap();
	drop(trie);

	let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
		.with_recorder(&mut recorder)
		.build();

	assert_eq!(trie.get(&commitment_key).unwrap().unwrap(), commitment_value);

	let proof = recorder
		.drain()
		.into_iter()
		.map(|f| f.data)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	// 2. first set the state commitment using sudo set_storage
	let state_commitment =
		StateCommitment { timestamp: 0, overlay_root: Some(root), state_root: root };
	let height = StateMachineHeight {
		id: StateMachineId {
			state_id: StateMachine::Polkadot(para_id).into(),
			consensus_state_id: *b"PARA",
		},
		height: 200,
	};
	let address1 = api::storage().ismp().state_commitments(&height);
	let address2 = api::storage().ismp().state_machine_update_time(&height);
	let key1 = client.storage().address_bytes(&address1)?;
	let key2 = client.storage().address_bytes(&address2)?;
	let start = SystemTime::now();
	let now = start.duration_since(UNIX_EPOCH).expect("Time went backwards");

	let call = RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_storage {
		items: vec![(key1, state_commitment.encode()), (key2, now.as_secs().encode())],
	});
	let call = client.tx().call_data(&api::tx().sudo().sudo(call))?;
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

	// create a block
	let _ = client
		.rpc()
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	// sanity check that it was properly stored
	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address1)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(item, state_commitment);

	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address2)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(item, now.as_secs());

	let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
		hasher: HashAlgorithm::Keccak,
		storage_proof: proof,
	})
	.encode();
	let proof = Proof { height, proof };

	// 3. next send the requests
	let tx = api::tx().ismp().handle_unsigned(vec![Message::Request(RequestMessage {
		requests: vec![post.clone().into()],
		proof: proof.clone(),
		signer: H256::random().as_bytes().to_vec(),
	})]);

	let error = client.tx().create_unsigned(&tx)?.submit_and_watch().await.unwrap_err();
	let subxt::Error::Rpc(RpcError::ClientError(err)) = error else {
		panic!("Unexpected error kind: {error:?}")
	};
	let jsonrpsee_error = err.downcast::<subxt_utils::client::RpcError>().unwrap();
	let subxt_utils::client::RpcError::RpcError(jsonrpsee_core::ClientError::Call(error)) =
		*jsonrpsee_error
	else {
		panic!("Unexpected error kind: {jsonrpsee_error:?}")
	};
	assert_eq!(error.message(), "Invalid Transaction");

	Ok(())
}

#[tokio::test]
#[ignore]
async fn test_will_reject_partially_paid_requests() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let client = subxt_utils::client::ws_client::<Hyperbridge>(
		&format!("ws://127.0.0.1:{}", port),
		u32::MAX,
	)
	.await?;

	let unit = 1_000_000_000_000u128;
	let per_byte_fee = 10 * unit;
	let para_id = 3000u32;
	// 1. initialize the ismp parachain client by adding the whitelisted paraId
	{
		let calls = vec![
			RuntimeCall::IsmpParachain(
				runtime_types::ismp_parachain::pallet::Call::add_parachain {
					para_ids: vec![para_id],
				},
			),
			// init the host executive
			RuntimeCall::HostExecutive(
				runtime_types::pallet_ismp_host_executive::pallet::Call::set_host_params {
					params: vec![
						(
							StateMachine::Polkadot(para_id).into(),
							runtime_types::pallet_ismp_host_executive::params::HostParam::SubstrateHostParam(
								runtime_types::pallet_hyperbridge::VersionedHostParams::V1(per_byte_fee)
							)
						)
					],
				},
			),
		];
		let call =
			RuntimeCall::Utility(runtime_types::pallet_utility::pallet::Call::batch_all { calls });
		let call = client.tx().call_data(&api::tx().sudo().sudo(call))?;
		let extrinsic: Bytes = client
			.rpc()
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await?;
		let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = client
			.rpc()
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;

		let finalized = client
			.rpc()
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	let post = Post {
		source: StateMachine::Polkadot(para_id),
		dest: StateMachine::Polygon,
		nonce: 0,
		from: H256::random().as_bytes().to_vec(),
		to: H256::random().as_bytes().to_vec(),
		timeout_timestamp: 0,
		data: H256::random().as_bytes().to_vec(),
	};
	let request = Request::Post(post.clone());

	let commitment = hash_request::<Keccak256>(&request);
	let mut db = <MemoryDB<KeccakHasher>>::default();
	let mut root = Default::default();
	let commitment_key = pallet_ismp::child_trie::request_commitment_storage_key(commitment);
	let payment_key = pallet_hyperbridge::child_trie::request_payment_storage_key(commitment);
	let commitment_value = H256::random().as_bytes().to_vec();
	let len = post.data.len() as u128 / 2;
	let payment_value = (len * per_byte_fee).encode();
	{
		let mut trie = TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
		trie.insert(&commitment_key, &commitment_value).unwrap();
		trie.insert(&payment_key, &payment_value).unwrap();
	}
	let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
		.with_recorder(&mut recorder)
		.build();

	assert_eq!(trie.get(&commitment_key).unwrap().unwrap(), commitment_value);
	assert_eq!(trie.get(&payment_key).unwrap().unwrap(), payment_value);

	let proof = recorder
		.drain()
		.into_iter()
		.map(|f| f.data)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	// 2. first set the state commitment using sudo set_storage
	let state_commitment =
		StateCommitment { timestamp: 0, overlay_root: Some(root), state_root: root };
	let height = StateMachineHeight {
		id: StateMachineId {
			state_id: StateMachine::Polkadot(para_id).into(),
			consensus_state_id: *b"PARA",
		},
		height: 200,
	};
	let address1 = api::storage().ismp().state_commitments(&height);
	let address2 = api::storage().ismp().state_machine_update_time(&height);
	let key1 = client.storage().address_bytes(&address1)?;
	let key2 = client.storage().address_bytes(&address2)?;
	let start = SystemTime::now();
	let now = start.duration_since(UNIX_EPOCH).expect("Time went backwards");

	let call = RuntimeCall::System(runtime_types::frame_system::pallet::Call::set_storage {
		items: vec![(key1, state_commitment.encode()), (key2, now.as_secs().encode())],
	});
	let call = client.tx().call_data(&api::tx().sudo().sudo(call))?;
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

	// create a block
	let _ = client
		.rpc()
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	// sanity check that it was properly stored
	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address1)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(item, state_commitment);

	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch(&address2)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(item, now.as_secs());

	let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
		hasher: HashAlgorithm::Keccak,
		storage_proof: proof,
	})
	.encode();
	let proof = Proof { height, proof };

	// 3. next send the requests
	let tx = api::tx().ismp().handle_unsigned(vec![Message::Request(RequestMessage {
		requests: vec![post.clone().into()],
		proof: proof.clone(),
		signer: H256::random().as_bytes().to_vec(),
	})]);

	let error = client.tx().create_unsigned(&tx)?.submit_and_watch().await.unwrap_err();
	let subxt::Error::Rpc(RpcError::ClientError(err)) = error else {
		panic!("Unexpected error kind: {error:?}")
	};
	let jsonrpsee_error = err.downcast::<subxt_utils::client::RpcError>().unwrap();
	let subxt_utils::client::RpcError::RpcError(jsonrpsee_core::ClientError::Call(error)) =
		*jsonrpsee_error
	else {
		panic!("Unexpected error kind: {jsonrpsee_error:?}")
	};
	assert_eq!(error.message(), "Invalid Transaction");

	Ok(())
}
