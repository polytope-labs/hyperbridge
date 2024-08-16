#![cfg(test)]
#![deny(missing_docs, unused_imports)]

use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::{Event, StateMachineUpdated},
	host::StateMachine,
	messaging::{Message, Proof, ResponseMessage},
	router::{Request, RequestResponse},
};

use pallet_hyperbridge::VersionedHostParams;
use pallet_ismp_host_executive::HostParam;
use sc_service::TaskManager;
use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
use subxt::{
	ext::{
		codec::{Decode, Encode},
		sp_core::{sr25519::Pair, Pair as PairT},
	},
	tx::PairSigner,
	utils::AccountId32,
};
use subxt_signer::sr25519::dev::{self};
use subxt_utils::{
	gargantua::api::{
		self,
		ismp::events::{PostRequestHandled, Request as RequestEvent},
		ismp_demo::events::GetResponse,
		runtime_types::{
			ismp::host::StateMachine as StateMachineType,
			pallet_ismp_demo::pallet::{GetRequest, TransferParams},
		},
	},
	relayer_nonce_storage_key, Hyperbridge,
};
//use subxt_utils::gargantua::api::host_executive::events::HostParamsSet;
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
	tx_block_height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let (client_a, client_b) =
		(chain_a_sub_client.clone().client, chain_b_sub_client.clone().client);

	let (chain_a_client, chain_b_client) =
		(Arc::new(chain_a_sub_client.clone()), Arc::new(chain_b_sub_client));

	// ====================== get the GET_REQUEST ===============================
	let mut get_request = None;
	let mut finalized_blocks_b_stream = client_a.blocks().subscribe_finalized().await?;

	while let Some(block_stream) = finalized_blocks_b_stream.next().await {
		match block_stream {
			Ok(block) => {
				let state_machine_update = StateMachineUpdated {
					state_machine_id: chain_a_client.state_machine_id(),
					latest_height: (block.number() as u64) + 1,
				};

				if let Ok(events) = chain_a_client
					.query_ismp_events(tx_block_height - 1, state_machine_update)
					.await
				{
					if let Some(event) = events.into_iter().find_map(|event| match event {
						Event::GetRequest(_) => Some(event),
						_ => None,
					}) {
						get_request = Some(event.clone());
						break
					} else {
						continue
					}
				} else {
					continue
				}
			},
			Err(err) => panic!("No next block in the stream {:?}", err),
		}
	}
	// ======== process the request offchain ( 1. Make the response ) =============
	let response = {
		let get = match get_request.unwrap() {
			Event::GetRequest(get) => get,
			_ => panic!("Not supported"),
		};
		let dest_chain_block_hash = client_b.rpc().block_hash(Some(get.height.into())).await?;
		let keys = get.keys.iter().map(|key| &key[..]).collect::<Vec<&[u8]>>();
		let value_proof = client_b.rpc().read_proof(keys, dest_chain_block_hash).await?;
		let proof = value_proof.proof.into_iter().map(|bytes| bytes.0).collect::<Vec<Vec<u8>>>();

		let proof_of_value = SubstrateStateProof::StateProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: proof,
		});
		let proof = Proof {
			height: StateMachineHeight {
				id: chain_b_client.state_machine_id(),
				height: get.height,
			},
			proof: proof_of_value.encode(),
		};
		let response = ResponseMessage {
			datagram: RequestResponse::Request(vec![Request::Get(get.clone())]),
			proof,
			signer: chain_a_client.address(), // both A&B have same relayer address
		};

		Message::Response(response)
	};
	// =================== send to the source chain ================================
	let _res = chain_a_client.submit(vec![response]).await?;
	//==================== after approx 7-9 blocks the response event is emitted ===
	// =================== fetch the returned value ================================

	let mut response_event: Option<GetResponse> = None;

	let mut finalized_blocks_stream = client_a.blocks().subscribe_finalized().await?;
	while let Some(block) = finalized_blocks_stream.next().await {
		match block {
			Ok(block) => {
				let block_hash = block.hash();
				let fetched_events =
					client_a.events().at(block_hash).await?.find_first::<GetResponse>()?;
				match fetched_events {
					Some(res_event) => {
						response_event = Some(res_event.clone());
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

	let encoded_value = match response_event.unwrap().0[0].clone() {
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
	let bob_signer = PairSigner::<Hyperbridge, _>::new(
		Pair::from_string("//Bob", None).expect("Unable to create Bob account"),
	);
	let bob_key = api::storage().system().account(AccountId32(dev::bob().public_key().0));

	let amount = 100_000 * 1000000000000;
	let transfer_call = api::tx().ismp_demo().transfer(TransferParams {
		to: AccountId32(dev::bob().public_key().0),
		amount,
		para_id: 2001,
		timeout: 80,
	});

	let bob_chain_a_initial_balance = client_a
		.storage()
		.at_latest()
		.await?
		.fetch(&bob_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	let bob_chain_b_initial_balance = client_b
		.storage()
		.at_latest()
		.await?
		.fetch(&bob_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	let result = client_a
		.tx()
		.sign_and_submit_then_watch_default(&transfer_call, &bob_signer)
		.await?
		.wait_for_finalized_success()
		.await?
		.all_events_in_block()
		.clone();

	let tx_block_hash = result.block_hash();

	let events = client_a.events().at(tx_block_hash).await?;
	log::info!("Ismp Events: {:?} \n", events.find_last::<RequestEvent>()?);

	// Assert burnt & transferred tokens in chain A
	let bob_chain_a_new_balance = client_a
		.storage()
		.at_latest()
		.await?
		.fetch(&bob_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

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
	log::info!("Chain B Event: {:?}", post_request_handled_event);
	// The relayer should finish sending the request message to chain B

	let bob_chain_b_new_balance = client_b
		.storage()
		.at_latest()
		.await?
		.fetch(&bob_key)
		.await?
		.ok_or("Failed to fetch")
		.unwrap()
		.data
		.free;

	// diving by 100000000000 for better assertion as the rem balance = initial - amount - fees
	// in chain A
	assert_eq!(
		(bob_chain_a_initial_balance - amount) / 1000000000000,
		bob_chain_a_new_balance / 1000000000000
	);
	// in chain B
	assert_eq!(
		(bob_chain_b_initial_balance + amount) / 1000000000000,
		bob_chain_b_new_balance / 1000000000000
	);

	Ok(())
}

/// fetch a foreign storage item from a given key
async fn get_request_works() -> Result<(), anyhow::Error> {
	let _ = log_setup();
	let (chain_a_sub_client, chain_b_sub_client) = create_clients().await?;

	log::info!(" \n 🧊integration test for para: 2000 to para 2001: get request \n");

	// =======================================================================
	let (chain_a_client, chain_b_client) =
		(Arc::new(chain_a_sub_client.clone()), Arc::new(chain_b_sub_client.clone()));

	let (client_a, client_b) =
		(chain_a_sub_client.clone().client, chain_b_sub_client.clone().client);

	// Accounts & keys
	let dave_signer = PairSigner::<Hyperbridge, _>::new(
		Pair::from_string("//Dave", None).expect("Unable to create Dave account"),
	);
	// parachain info pallet fetching para id
	let encoded_chain_b_id_storage_key =
		"0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";

	let latest_height_b =
		chain_a_client.query_latest_height(chain_b_client.state_machine_id()).await? - 5;

	let get_request = api::tx().ismp_demo().get_request(GetRequest {
		para_id: 2001,
		height: latest_height_b,
		timeout: 0,
		keys: vec![hex::decode(encoded_chain_b_id_storage_key.strip_prefix("0x").unwrap()).unwrap()],
	});

	let tx_result = client_a
		.tx()
		.sign_and_submit_then_watch_default(&get_request, &dave_signer)
		.await?
		.wait_for_finalized_success()
		.await?
		.all_events_in_block()
		.clone();

	let tx_block_hash = tx_result.block_hash();
	let tx_block_height = client_a.blocks().at(tx_block_hash).await?.number() as u64;
	let events = client_a.events().at(tx_block_hash).await?;
	let event = events.find_last::<RequestEvent>()?.unwrap();
	log::info!("Ismp Events: {:?} \n", event);

	// ======================= handle the get request and resubmit to chain A (origin chain)
	// =====================================

	let value_returned_encoded =
		relay_get_response_message(chain_a_sub_client, chain_b_sub_client, tx_block_height).await?;

	let para_id_chain_b: u32 = Decode::decode(&mut &value_returned_encoded[..])?;

	let fetched_para_id_chain_b = client_b
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
	let (chain_a_sub_client, chain_b_sub_client) = create_clients().await?;
	let (client_a, _client_b) =
		(chain_a_sub_client.clone().client, chain_b_sub_client.clone().client);

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

#[tokio::test]
#[ignore]
async fn test_raw_storage_queries() -> Result<(), anyhow::Error> {
	use tesseract_primitives::HyperbridgeClaim;
	let chain_a_config = SubstrateConfig {
		state_machine: StateMachine::Kusama(4009),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9001".to_string(), // url from local-testnet zombienet config
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		latest_height: None,
		max_concurent_queries: None,
	};

	let mut client = SubstrateClient::<KeccakSubstrateChain>::new(chain_a_config.clone()).await?;
	client.address = hex::decode("bc50b90751bfcccbfa4c7220261909d0f528b00f").unwrap();

	// Query relayer balance

	let balance = client
		.available_amount(Arc::new(client.clone()), &StateMachine::Evm(97))
		.await?;

	dbg!(balance);

	dbg!(hex::encode(client.address()));
	let key = relayer_nonce_storage_key(client.address(), StateMachine::Evm(97));
	let raw_value = client.client.storage().at_latest().await?.fetch_raw(&key).await?.unwrap();
	let nonce: u64 = Decode::decode(&mut &*raw_value)?;

	dbg!(nonce);

	let height = StateMachineHeight {
		id: StateMachineId { state_id: StateMachine::Evm(97), consensus_state_id: *b"BSC0" },
		height: 42858376,
	};

	let state_commitment = client.query_state_machine_commitment(height).await?;

	dbg!(state_commitment);

	let update_time = client.query_state_machine_update_time(height).await?;

	dbg!(update_time);

	let host_params = client.query_host_params(StateMachine::Evm(97)).await?;

	dbg!(host_params);

	Ok(())
}
