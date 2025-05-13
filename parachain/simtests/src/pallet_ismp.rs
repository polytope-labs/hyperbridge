#![cfg(test)]

use std::{
	collections::HashSet,
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
use subxt::{error::RpcError, rpc_params, tx::SubmittableExtrinsic};
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

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
	state_machine_commitment_storage_key, state_machine_update_time_storage_key, Extrinsic,
	Hyperbridge,
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
	let client = subxt_utils::client::ws_client::<Hyperbridge>(
		&format!("ws://127.0.0.1:{}", port),
		u32::MAX,
	)
	.await?;

	let para_id = 3000u32;
	let slot_duration = 6000u64;

	// 1. initialize the ismp parachain client by adding the whitelisted paraId
	{
		let add_parachain_call = Extrinsic::new(
			"IsmpParachain",
			"add_parachain",
			vec![ParachainData { id: para_id, slot_duration }].encode(),
		);
		let sudo_call = Extrinsic::new("Sudo", "sudo", client.tx().call_data(&add_parachain_call)?);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = client
			.rpc()
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
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

	// Init the host executive extrinsic
	{
		let set_host_params_call = Extrinsic::new(
			"HostExecutive",
			"set_host_params",
			vec![(
				StateMachine::Kusama(para_id),
				HostParam::SubstrateHostParam(VersionedHostParams::V1(SubstrateHostParams {
					default_per_byte_fee: 0u128,
					..Default::default()
				})),
			)]
			.encode(),
		);
		let sudo_call =
			Extrinsic::new("Sudo", "sudo", client.tx().call_data(&set_host_params_call)?);
		let call = client.tx().call_data(&sudo_call)?;
		let extrinsic: Bytes = client
			.rpc()
			.request(
				"simnode_authorExtrinsic",
				// author an extrinsic from alice, the sudo account
				rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
			)
			.await
			.map_err(|err| println!("{:?}", err))
			.expect("REASON");
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

	let post = PostRequest {
		source: StateMachine::Kusama(para_id),
		dest: StateMachine::Evm(8002),
		nonce: 0,
		from: H256::random().as_bytes().to_vec(),
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
		id: StateMachineId {
			state_id: StateMachine::Kusama(para_id).into(),
			consensus_state_id: *b"PAS0",
		},
		height: 200,
	};

	let key1 = state_machine_commitment_storage_key(height);
	let key2 = state_machine_update_time_storage_key(height);
	let start = SystemTime::now();
	let now = start.duration_since(UNIX_EPOCH).expect("Time went backwards");

	let call = Extrinsic::new(
		"System",
		"set_storage",
		vec![(key1.clone(), state_commitment.encode()), (key2.clone(), now.as_secs().encode())]
			.encode(),
	);
	let sudo_call = Extrinsic::new("Sudo", "sudo", client.tx().call_data(&call)?);
	let call = client.tx().call_data(&sudo_call)?;
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
		.fetch_raw(&key1)
		.await?
		.ok_or_else(|| anyhow!("Failed to set state commitment"))?;

	assert_eq!(state_commitment, Decode::decode(&mut &*item)?);

	let item = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(&key2)
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

	// 3. next send the requests
	let tx = Extrinsic::new(
		"Ismp",
		"handle_unsigned",
		vec![Message::Request(RequestMessage {
			requests: vec![post.clone().into()],
			proof: proof.clone(),
			signer: H256::random().as_bytes().to_vec(),
		})]
		.encode(),
	);

	// send once
	let progress = client.tx().create_unsigned(&tx)?.submit_and_watch().await?;
	// send twice, txpool should reject it
	{
		let tx = Extrinsic::new(
			"Ismp",
			"handle_unsigned",
			vec![Message::Request(RequestMessage {
				requests: vec![post.clone().into()],
				proof: proof.clone(),
				signer: H256::random().as_bytes().to_vec(),
			})]
			.encode(),
		);
		let error = client.tx().create_unsigned(&tx)?.submit_and_watch().await.unwrap_err();
		let subxt::Error::Rpc(RpcError::ClientError(_err)) = error else {
			panic!("Unexpected error kind: {error:?}")
		};
	};

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

	// send after block inclusion, txpool should reject it
	{
		let tx = Extrinsic::new(
			"Ismp",
			"handle_unsigned",
			vec![Message::Request(RequestMessage {
				requests: vec![post.clone().into()],
				proof: proof.clone(),
				signer: H256::random().as_bytes().to_vec(),
			})]
			.encode(),
		);
		let error = client.tx().create_unsigned(&tx)?.submit_and_watch().await.unwrap_err();
		let subxt::Error::Rpc(RpcError::ClientError(_err)) = error else {
			panic!("Unexpected error kind: {error:?}")
		};
	};

	Ok(())
}
