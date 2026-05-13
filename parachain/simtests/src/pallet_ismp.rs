#![cfg(test)]

use std::{
	collections::{BTreeMap, HashSet},
	env,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp_parachain::ParachainData;
use pallet_hyperbridge::{SubstrateHostParams, VersionedHostParams};
use pallet_ismp_host_executive::HostParam;
use polkadot_sdk::*;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, keccak_256, Bytes, KeccakHasher};
use sp_keyring::sr25519::Keyring;
use sp_trie::{LayoutV0, MemoryDB};
use subxt::{
	backend::legacy::LegacyRpcMethods, error::RpcError, ext::subxt_rpcs::rpc_params,
	tx::SubmittableTransaction,
};
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

use crypto_utils::verification::Signature;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, Message, Proof, RequestMessage},
	router::{PostRequest, Request},
};
use pallet_ismp::child_trie::{self};
use primitive_types::H256;
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
use subxt_utils::{
	state_machine_commitment_storage_key, state_machine_update_time_storage_key,
	values::{
		host_params_btreemap_to_value, messages_to_value, parachain_data_to_value,
		state_machine_to_value, storage_kv_list_to_value,
	},
	BlakeSubstrateChain, Hyperbridge,
};

#[derive(Clone, Default)]
pub struct Keccak256;

impl ismp::messaging::Keccak256 for Keccak256 {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		keccak_256(bytes).into()
	}
}

#[tokio::test]
#[ignore]
async fn test_txpool_should_reject_duplicate_requests() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;
	let _rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(rpc_client.clone());

	let para_id = 3000u32;
	let slot_duration = 6000u64;
	let source = StateMachine::Kusama(para_id);
	// Bytes used as the `from` field of the POST request below. Pre-computed
	// here so the same value can be added to the bandwidth allowlist during
	// setup.
	let from = H256::random().as_bytes().to_vec();

	// 1. initialize the ismp parachain client by adding the whitelisted paraId
	{
		let add_parachain_call = subxt::dynamic::tx(
			"IsmpParachain",
			"add_parachain",
			vec![vec![parachain_data_to_value(&ParachainData { id: para_id, slot_duration })]],
		);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![add_parachain_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	// Init the host executive extrinsic
	{
		let mut host_params = BTreeMap::new();
		host_params.insert(
			source,
			HostParam::SubstrateHostParam(VersionedHostParams::V1(SubstrateHostParams {
				default_per_byte_fee: 0u128,
				..Default::default()
			})),
		);

		let host_params_value = host_params_btreemap_to_value(&host_params);

		let set_host_params_call =
			subxt::dynamic::tx("HostExecutive", "set_host_params", vec![host_params_value]);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![set_host_params_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	// Add the module used in `post.from` to pallet-bandwidth's Allowlist for
	// the source chain so that the bandwidth gate accepts the request without
	// requiring a prepaid subscription.
	{
		let set_allowlist_call = subxt::dynamic::tx(
			"Bandwidth",
			"set_allowlist",
			vec![
				state_machine_to_value(&source),
				subxt::dynamic::Value::from_bytes(from.clone()),
				subxt::dynamic::Value::bool(true),
			],
		);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![set_allowlist_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	let post = PostRequest {
		source,
		dest: StateMachine::Evm(8002),
		nonce: 0,
		from: from.clone(),
		to: H256::random().as_bytes().to_vec(),
		timeout_timestamp: 0,
		body: H256::random().as_bytes().to_vec(),
	};
	let request = Request::Post(post.clone());

	let commitment = hash_request::<Keccak256>(&request);
	let mut db = <MemoryDB<KeccakHasher>>::default();
	let mut root = Default::default();
	let mut trie = TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
	let key = child_trie::request_commitment_storage_key(commitment);
	let value = H256::random().as_bytes().to_vec();
	trie.insert(&key, &value).unwrap();
	drop(trie);

	let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
		.with_recorder(&mut recorder)
		.build();

	assert_eq!(trie.get(&key).unwrap().unwrap(), value);

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
		id: StateMachineId { state_id: source.into(), consensus_state_id: *b"PAS0" },
		height: 200,
	};

	let key1 = state_machine_commitment_storage_key(height);
	let key2 = state_machine_update_time_storage_key(height);
	let start = SystemTime::now();
	let now = start.duration_since(UNIX_EPOCH).expect("Time went backwards");

	let kv_list =
		vec![(key1.clone(), state_commitment.encode()), (key2.clone(), now.as_secs().encode())];

	let call =
		subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv_list)]);
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);
	let call = client.tx().call_data(&sudo_call)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			// author an extrinsic from alice
			rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;

	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);

	progress.wait_for_finalized_success().await?;

	// create a block
	let _ = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	// sanity check that it was properly stored
	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(key1.clone())
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(state_commitment, Decode::decode(&mut &*item)?);

	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(key2.clone())
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;
	let update_time: u64 = Decode::decode(&mut &*item)?;
	assert_eq!(now.as_secs(), update_time);

	let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
		hasher: HashAlgorithm::Keccak,
		storage_proof: proof,
	})
	.encode();
	let proof = Proof { height, proof };

	let signature = Signature::Sr25519 {
		public_key: H256::random().as_bytes().to_vec(),
		signature: H256::random().as_bytes().to_vec(),
	};

	// 3. next send the requests
	let tx = subxt::dynamic::tx(
		"Ismp",
		"handle_unsigned",
		vec![messages_to_value(vec![Message::Request(RequestMessage {
			requests: vec![post.clone().into()],
			proof: proof.clone(),
			signer: signature.encode(),
		})])],
	);

	// send once
	let progress = client.tx().create_unsigned(&tx)?.submit_and_watch().await?;
	// send twice, txpool should reject it
	{
		let tx = subxt::dynamic::tx(
			"Ismp",
			"handle_unsigned",
			vec![messages_to_value(vec![Message::Request(RequestMessage {
				requests: vec![post.clone().into()],
				proof: proof.clone(),
				signer: signature.encode(),
			})])],
		);
		let error = client.tx().create_unsigned(&tx)?.submit_and_watch().await.unwrap_err();
		let subxt::Error::Rpc(RpcError::ClientError(_err)) = error else {
			panic!("Unexpected error kind: {error:?}")
		};
	};

	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;

	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;

	// send after block inclusion, txpool should reject it
	{
		let tx = subxt::dynamic::tx(
			"Ismp",
			"handle_unsigned",
			vec![messages_to_value(vec![Message::Request(RequestMessage {
				requests: vec![post.clone().into()],
				proof: proof.clone(),
				signer: signature.encode(),
			})])],
		);
		let error = client.tx().create_unsigned(&tx)?.submit_and_watch().await.unwrap_err();
		let subxt::Error::Rpc(RpcError::ClientError(_err)) = error else {
			panic!("Unexpected error kind: {error:?}")
		};
	};

	Ok(())
}

/// Configure a bandwidth tier, force-credit a subscription for the module
/// that will appear in `post.from`, and verify that the bandwidth gate
/// admits the request through the prepaid subscription path rather than the
/// allowlist bypass exercised in [`test_txpool_should_reject_duplicate_requests`].
#[tokio::test]
#[ignore]
async fn test_force_credited_bandwidth_satisfies_gate() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;
	let _rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(rpc_client.clone());

	// Use a distinct para_id from the duplicate-rejection test so the two
	// tests can run back-to-back against the same simnode without colliding
	// on the parachain-registration step.
	let para_id = 3001u32;
	let slot_duration = 6000u64;
	let source = StateMachine::Kusama(para_id);
	let from = H256::random().as_bytes().to_vec();

	// 1. initialize the ismp parachain client by adding the whitelisted paraId
	{
		let add_parachain_call = subxt::dynamic::tx(
			"IsmpParachain",
			"add_parachain",
			vec![vec![parachain_data_to_value(&ParachainData { id: para_id, slot_duration })]],
		);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![add_parachain_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	// Init the host executive extrinsic
	{
		let mut host_params = BTreeMap::new();
		host_params.insert(
			source,
			HostParam::SubstrateHostParam(VersionedHostParams::V1(SubstrateHostParams {
				default_per_byte_fee: 0u128,
				..Default::default()
			})),
		);

		let host_params_value = host_params_btreemap_to_value(&host_params);

		let set_host_params_call =
			subxt::dynamic::tx("HostExecutive", "set_host_params", vec![host_params_value]);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![set_host_params_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	// Configure TierOne as an active SKU. Pallet-bandwidth's `set_tier`
	// rejects zero values, so a real (bytes, duration_secs) pair is needed
	// even though `force_credit` doesn't read this row when crediting.
	let tier_one = || {
		subxt::dynamic::Value::variant(
			"TierOne",
			subxt::ext::scale_value::Composite::unnamed(vec![]),
		)
	};
	let bandwidth_bytes: u128 = 10 * 1024 * 1024;
	let duration_secs: u64 = 7 * 24 * 60 * 60;
	{
		let tier_config = subxt::dynamic::Value::named_composite(vec![
			("bytes", subxt::dynamic::Value::u128(bandwidth_bytes)),
			("duration_secs", subxt::dynamic::Value::u128(duration_secs.into())),
		]);
		let tier_config_some = subxt::dynamic::Value::variant(
			"Some",
			subxt::ext::scale_value::Composite::unnamed(vec![tier_config]),
		);
		let set_tier_call =
			subxt::dynamic::tx("Bandwidth", "set_tier", vec![tier_one(), tier_config_some]);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![set_tier_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	// Force-credit a fresh subscription on (source, from). The gate now
	// admits requests from this module up to `bandwidth_bytes` bytes within
	// the subscription window.
	{
		let force_credit_params = subxt::dynamic::Value::named_composite(vec![
			("app_chain", state_machine_to_value(&source)),
			("app", subxt::dynamic::Value::from_bytes(from.clone())),
			("tier", tier_one()),
			("bytes", subxt::dynamic::Value::u128(bandwidth_bytes)),
			("duration_secs", subxt::dynamic::Value::u128(duration_secs.into())),
		]);
		let force_credit_call =
			subxt::dynamic::tx("Bandwidth", "force_credit", vec![force_credit_params]);
		let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![force_credit_call.into_value()]);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = rpc_client
			.request(
				"simnode_authorExtrinsic",
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
		let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
		let progress = submittable.submit_and_watch().await?;
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		let finalized = rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
		assert!(finalized);
		progress.wait_for_finalized_success().await?;
	}

	let post = PostRequest {
		source,
		dest: StateMachine::Evm(8002),
		nonce: 0,
		from: from.clone(),
		to: H256::random().as_bytes().to_vec(),
		timeout_timestamp: 0,
		body: H256::random().as_bytes().to_vec(),
	};
	let request = Request::Post(post.clone());

	let commitment = hash_request::<Keccak256>(&request);
	let mut db = <MemoryDB<KeccakHasher>>::default();
	let mut root = Default::default();
	let mut trie = TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
	let key = child_trie::request_commitment_storage_key(commitment);
	let value = H256::random().as_bytes().to_vec();
	trie.insert(&key, &value).unwrap();
	drop(trie);

	let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
		.with_recorder(&mut recorder)
		.build();
	assert_eq!(trie.get(&key).unwrap().unwrap(), value);

	let proof = recorder
		.drain()
		.into_iter()
		.map(|f| f.data)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	// Seed the state commitment for `source` at a fixed height so the
	// ParachainConsensusClient on the runtime accepts the membership proof
	// against the in-memory trie root we just constructed.
	let state_commitment =
		StateCommitment { timestamp: 0, overlay_root: Some(root), state_root: root };
	let height = StateMachineHeight {
		id: StateMachineId { state_id: source.into(), consensus_state_id: *b"PAS0" },
		height: 200,
	};
	let key1 = state_machine_commitment_storage_key(height);
	let key2 = state_machine_update_time_storage_key(height);
	let start = SystemTime::now();
	let now = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
	let kv_list =
		vec![(key1.clone(), state_commitment.encode()), (key2.clone(), now.as_secs().encode())];

	let call =
		subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv_list)]);
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);
	let call = client.tx().call_data(&sudo_call)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;

	let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
		hasher: HashAlgorithm::Keccak,
		storage_proof: proof,
	})
	.encode();
	let proof = Proof { height, proof };

	let signature = Signature::Sr25519 {
		public_key: H256::random().as_bytes().to_vec(),
		signature: H256::random().as_bytes().to_vec(),
	};

	// Submitting the request should succeed: the bandwidth gate finds a
	// live subscription on (source, from) credited above and admits it.
	let tx = subxt::dynamic::tx(
		"Ismp",
		"handle_unsigned",
		vec![messages_to_value(vec![Message::Request(RequestMessage {
			requests: vec![post.clone().into()],
			proof: proof.clone(),
			signer: signature.encode(),
		})])],
	);
	let progress = client.tx().create_unsigned(&tx)?.submit_and_watch().await?;
	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;

	Ok(())
}
