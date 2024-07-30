#![cfg(test)]
#![deny(missing_docs, unused_imports)]

use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{Message, Proof, ResponseMessage},
	router::{Request, RequestResponse},
};
use pallet_hyperbridge::VersionedHostParams;
use pallet_ismp_demo as IsmpPalletDemo;
use pallet_ismp_host_executive::HostParam;
use sc_service::TaskManager;
use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
use subxt::{
	ext::codec::{Decode, Encode},
	utils::AccountId32,
};
use subxt_signer::sr25519::dev::{self};
use subxt_utils::{
	gargantua::{
		api,
		api::{
			ismp::events::{PostRequestHandled, Request as RequestEventStatic},
			runtime_types::ismp::host::StateMachine as StateMachineType,
		},
	},
	Hyperbridge,
};
use tesseract::logging::setup as log_setup;
use tesseract_messaging::relay;
use tesseract_primitives::{config::RelayerConfig, IsmpProvider};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient, SubstrateConfig};
use transaction_fees::TransactionPayment;

/// This function is used to fetch get request and construct get response and submit to chain
/// A(source chain)
async fn relay_get_response_message(
	chain_a_sub_client: SubstrateClient<Hyperbridge>,
	chain_b_sub_client: SubstrateClient<Hyperbridge>,
	get_request: ismp::router::GetRequest,
) -> Result<Vec<u8>, anyhow::Error> {
	let (chain_a_client, chain_b_client) =
		(Arc::new(chain_a_sub_client.clone()), Arc::new(chain_b_sub_client));

	// ======== process the request offchain ( 1. Make the response ) =============
	let response = {
		let dest_chain_block_hash =
			chain_b_client.client.rpc().block_hash(Some(get_request.height.into())).await?;
		let keys = get_request.keys.iter().map(|key| &key[..]).collect::<Vec<&[u8]>>();
		let value_proof =
			chain_b_client.client.rpc().read_proof(keys, dest_chain_block_hash).await?;
		let proof = value_proof.proof.into_iter().map(|bytes| bytes.0).collect::<Vec<Vec<u8>>>();

		let proof_of_value = SubstrateStateProof::StateProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: proof,
		});
		let proof = Proof {
			height: StateMachineHeight {
				id: chain_b_client.state_machine_id(),
				height: get_request.height,
			},
			proof: proof_of_value.encode(),
		};
		let response = ResponseMessage {
			datagram: RequestResponse::Request(vec![Request::Get(get_request.clone())]),
			proof,
			signer: chain_a_client.address(), // both A&B have same relayer address
		};

		Message::Response(response)
	};
	// =================== send to the source chain ================================
	let _res = chain_a_client.submit(vec![response]).await?;
	// =================== fetch the returned value ================================
	let hashed_key: [u8; 16] =
		hex::decode("0x6acc85a7ed191beef75dc62a9eb8b353".strip_prefix("0x").unwrap())
			.unwrap()
			.try_into()
			.unwrap();
	let encoded_value = chain_a_client
		.client
		.storage()
		.at_latest()
		.await?
		.fetch(&api::storage().ismp_demo().get_responses(hashed_key))
		.await?
		.unwrap();

	let encoded_value = match encoded_value {
		Some(value) => value,
		None => {
			panic!("Value not found")
		},
	};

	Ok(encoded_value)
}

/// Configure the state machines and spawn the messaging relayer
async fn create_clients(
) -> Result<(SubstrateClient<Hyperbridge>, SubstrateClient<Hyperbridge>), anyhow::Error> {
	let chain_a_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(2000),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9990".to_string(), // url from local-testnet zombienet config
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		latest_height: None,
		max_concurent_queries: None,
	};

	let chain_b_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(2001),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9991".to_string(),
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		latest_height: None,
		max_concurent_queries: None,
	};

	// setup state machines
	let chain_a_sub_client =
		SubstrateClient::<KeccakSubstrateChain>::new(chain_a_config.clone()).await?;
	let chain_b_sub_client =
		SubstrateClient::<KeccakSubstrateChain>::new(chain_b_config.clone()).await?;

	Ok((chain_a_sub_client, chain_b_sub_client))
}

/// A function to set host params when the network is spawned for ismp messages execution to work
async fn set_host_params(
	chain_sub_client: SubstrateClient<Hyperbridge>,
) -> Result<(), anyhow::Error> {
	// set host params for the original chain 2000 of dest chain 2001
	if chain_sub_client.state_machine_id().state_id == StateMachine::Kusama(2000) {
		chain_sub_client
			.clone()
			.set_host_params(BTreeMap::from([(
				StateMachine::Kusama(2001),
				HostParam::SubstrateHostParam(VersionedHostParams::V1(0)),
			)]))
			.await?;
	} else {
		// set host params for the original chain 2001 of dest chain 2000
		chain_sub_client
			.clone()
			.set_host_params(BTreeMap::from([(
				StateMachine::Kusama(2000),
				HostParam::SubstrateHostParam(VersionedHostParams::V1(0)),
			)]))
			.await?;
	}
	Ok(())
}

/// Assertion is on ismp related events, and state changes on source and destination chain
/// Alice in chain A sends 100_000 tokens to Alice in chain B
async fn parachain_messaging() -> Result<(), anyhow::Error> {
	let _ = log_setup();
	let (chain_a_sub_client, chain_b_sub_client) = create_clients().await?;
	log::info!(
		"🧊integration test for para:{} to para {}: fund transfer",
		chain_a_sub_client.clone().state_machine_id().state_id,
		chain_b_sub_client.clone().state_machine_id().state_id
	);

	// initiate message relaying task
	let tx_payment = Arc::new(
		TransactionPayment::initialize(&"/tmp/dev.db") // out of hyperbridge directory
			.await
			.map_err(|err| anyhow!("Error initializing database: {err:?}"))?,
	);

	let tokio_handle = tokio::runtime::Handle::current();
	let task_manager = TaskManager::new(tokio_handle, None)?;
	let relayer_config = RelayerConfig::default();

	let (chain_a_client, chain_b_client) = (
		Arc::new(chain_a_sub_client.clone()) as Arc<dyn IsmpProvider>,
		Arc::new(chain_b_sub_client.clone()) as Arc<dyn IsmpProvider>,
	);

	let mut client_map = HashMap::new();
	client_map
		.insert(chain_a_sub_client.clone().state_machine_id().state_id, chain_a_client.clone());
	client_map
		.insert(chain_b_sub_client.clone().state_machine_id().state_id, chain_b_client.clone());

	let (client_a, client_b) =
		(chain_a_sub_client.clone().client, chain_b_sub_client.clone().client);

	relay(
		chain_a_sub_client.clone(),
		chain_b_client.clone(),
		relayer_config.clone(),
		StateMachine::Kusama(3000), // random coprocessor id
		tx_payment,
		client_map.clone(),
		&task_manager,
	)
	.await?;
	// =========================== Accounts & keys =====================================

	let alice_key = api::storage().system().account(AccountId32(dev::alice().public_key().0));

	let alice_chain_a_initial_balance = client_a
		.storage()
		.at_latest()
		.await?
		.fetch(&alice_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	let alice_chain_b_initial_balance = client_b
		.storage()
		.at_latest()
		.await?
		.fetch(&alice_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	let amount = 100_000 * 1000000000000;
	let transfer_params = IsmpPalletDemo::TransferParams {
		to: AccountId32(dev::alice().public_key().0),
		amount,
		para_id: 2001,
		timeout: 80,
	};

	let _tx_block_hash = chain_a_sub_client.transfer(transfer_params).await?;

	// watch for PostRequestHandled event in chain b
	let mut post_request_handled_event = None;

	let mut finalized_blocks_stream = client_b.blocks().subscribe_finalized().await?;
	while let Some(block) = finalized_blocks_stream.next().await {
		match block {
			Ok(block) => {
				let block_hash = block.hash();
				let fetched_events =
					client_b.events().at(block_hash).await?.find_first::<PostRequestHandled>()?;
				match fetched_events {
					Some(res_event) => {
						post_request_handled_event = Some(res_event.clone());
						break
					},
					None => continue,
				}
			},
			Err(err) => {
				panic!("Error in finalized block stream: {:?}", err)
			},
		}
	}
	log::info!("Chain B Event: {:?} \n", post_request_handled_event);
	// The relayer should finish sending the request message to chain B

	// Asset burnt & transferred tokens in chain A
	let alice_chain_a_new_balance = client_a
		.storage()
		.at_latest()
		.await?
		.fetch(&alice_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	// Asset minted
	let alice_chain_b_new_balance = client_b
		.storage()
		.at_latest()
		.await?
		.fetch(&alice_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	// diving by 100000000000 for better assertion as the rem balance = initial - amount - fees
	// in chain A assets were burnt
	assert_eq!(
		(alice_chain_a_initial_balance - amount) / 1000000000000,
		alice_chain_a_new_balance / 1000000000000
	);
	// in chain B the assets were minted
	assert_eq!(
		(alice_chain_b_initial_balance + amount) / 1000000000000,
		alice_chain_b_new_balance / 1000000000000
	);

	Ok(())
}

/// fetch a foreign storage item from a given key
async fn get_request_works() -> Result<(), anyhow::Error> {
	let _ = log_setup();
	let (chain_a_sub_client, chain_b_sub_client) = create_clients().await?;

	log::info!("🧊integration test for para: 2000 to para 2001: get request \n");

	// parachain info pallet fetching para id
	let encoded_chain_b_id_storage_key =
		"0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";
	let key = hex::decode(encoded_chain_b_id_storage_key.strip_prefix("0x").unwrap()).unwrap();

	let mut latest_fetch_height = chain_a_sub_client
		.state_machine_update_notification(chain_b_sub_client.state_machine_id())
		.await?
		.take(1)
		.next()
		.await
		.ok_or(anyhow!("No stream"))??
		.latest_height;

	let get_request_param = IsmpPalletDemo::GetRequest {
		para_id: 2001,
		height: latest_fetch_height as u32,
		timeout: 0,
		keys: vec![key.clone()],
	};

	let tx_block_hash = chain_a_sub_client.get_request(get_request_param).await?;

	let event = chain_a_sub_client
		.clone()
		.client
		.events()
		.at(tx_block_hash)
		.await?
		.find_first::<RequestEventStatic>()?
		.unwrap();

	let get_request_event = ismp::router::GetRequest {
		source: StateMachine::Kusama(2000),
		dest: StateMachine::Kusama(2001),
		nonce: event.request_nonce,
		from: pallet_ismp_demo::PALLET_ID.to_bytes(),
		keys: vec![key],
		height: latest_fetch_height,
		timeout_timestamp: 0,
	};
	// ====== handle the get request and resubmit to chain A (origin chain) ============

	let value_returned_encoded = relay_get_response_message(
		chain_a_sub_client,
		chain_b_sub_client.clone(),
		get_request_event,
	)
	.await?;

	let para_id_chain_b: u32 = Decode::decode(&mut &value_returned_encoded[..])?;

	let fetched_para_id_chain_b = chain_b_sub_client
		.client
		.storage()
		.at_latest()
		.await?
		.fetch(&api::storage().parachain_info().parachain_id())
		.await?
		.unwrap()
		.0;

	assert_eq!(para_id_chain_b, fetched_para_id_chain_b);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn run_integration_tests() -> Result<(), anyhow::Error> {
	let _ = log_setup();
	let (chain_a_sub_client, chain_b_sub_client) = create_clients().await?;
	let (client_a, _client_b) =
		(chain_a_sub_client.clone().client, chain_b_sub_client.clone().client);

	let mut block_stream = client_a.blocks().subscribe_finalized().await?;
	while let Some(block_result) = block_stream.next().await {
		match block_result {
			Ok(_block) => {
				log::info!("chain producing blocks and finalizing");
				break
			},
			Err(_err) => {
				log::info!("chain not yet producing blocks and finalizing");
				continue
			},
		}
	}
	//======================= run only once ( set host executives ) ====================
	// check if the host params are set
	let host_param = client_a
		.storage()
		.at_latest()
		.await?
		.fetch(&api::storage().host_executive().host_params(&StateMachineType::Kusama(2001)))
		.await?;
	if host_param.is_none() {
		set_host_params(chain_a_sub_client.clone()).await?;
		set_host_params(chain_b_sub_client.clone()).await?;
	}

	parachain_messaging().await?;
	get_request_works().await?;
	Ok(())
}
