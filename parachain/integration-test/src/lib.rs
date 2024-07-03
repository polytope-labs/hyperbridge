#![cfg(test)]
//#![deny(missing_docs, unused_imports)]

use anyhow::anyhow;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::{Event, StateMachineUpdated},
	host::{StateMachine, StateMachine::Kusama},
	messaging::{hash_request, Message, Proof, ResponseMessage},
	router::{Request::Get, RequestResponse},
};
use sc_service::TaskManager;
use std::{collections::HashMap, sync::Arc, time::Duration};
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
use subxt::{
	ext::{
		codec::Encode,
		sp_core::{sr25519::Pair, Pair as PairT},
	},
	tx::PairSigner,
	utils::AccountId32,
	OnlineClient,
};
use subxt_signer::sr25519::dev::{self};
use subxt_utils::{
	gargantua::{
		api,
		api::{
			ismp::events::Request,
			runtime_types::pallet_ismp_demo::pallet::{GetRequest, TransferParams},
		},
	},
	Hyperbridge,
};
use tesseract::logging::setup as log_setup;
use tesseract_config::AnyConfig;
use tesseract_messaging::relay;
use tesseract_primitives::{config::RelayerConfig, Hasher, IsmpProvider, Query};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient, SubstrateConfig};
use transaction_fees::TransactionPayment;

/// This function is used to fetch get request and construct get response and submit to chain
/// A(source chain)
async fn messaging_relayer_lite(
	client_b: OnlineClient<Hyperbridge>,
	chain_a_client: Arc<dyn IsmpProvider>,
	chain_b_client: Arc<dyn IsmpProvider>,
	chain_a_sub_client: SubstrateClient<Hyperbridge>,
	tx_block_height: u64,
) -> Result<(), anyhow::Error> {
	let state_machine_update = StateMachineUpdated {
		state_machine_id: chain_a_sub_client.state_machine_id(),
		latest_height: tx_block_height + 2,
	};
	let event_result_a = chain_a_client
		.query_ismp_events(tx_block_height - 2, state_machine_update.clone())
		.await?;

	// filter events
	// returns events for statemachine update of chain B
	let state_machine_update_event_a = event_result_a
		.clone()
		.into_iter()
		.filter(|event| match event {
			Event::StateMachineUpdated(state_machine_update) => true,
			_ => false,
		})
		.collect::<Vec<Event>>();

	// ====================== get the GET_REQUEST ===============================
	let request_events = event_result_a
		.into_iter()
		.filter(|event| match event {
			Event::GetRequestHandled(post) => true,
			Event::GetRequest(get) => true,
			_ => false,
		})
		.collect::<Vec<Event>>();
	log::info!("\n request_events: {:?} \n", request_events);

	// ======== process the request offchain ( 1. Make the response ) =============
	let get_request = request_events.get(0).unwrap();

	let response = {
		let get = match get_request {
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

		let response = ResponseMessage {
			datagram: RequestResponse::Request(vec![Get(get.clone())]),
			proof: Proof {
				height: StateMachineHeight {
					id: chain_b_client.state_machine_id(),
					height: get.height,
				},
				proof: proof_of_value.encode(),
			},
			signer: chain_b_client.address(),
		};

		Message::Response(response)
	};
	// =================== send to the source chain ================================
	let nn = chain_a_client.estimate_gas(vec![response.clone()]).await?;
	log::info!("Gas estimate: {:?}", nn);

	let res = chain_a_client.submit(vec![response]).await;
	match res {
		Ok(receipt) => {
			log::info!("Receipt: {:?}", receipt);
		},
		Err(err) => {
			log::info!("Error: {:?}", err);
		},
	}
	Ok(())
}

/// Configure the state machines and relayer
async fn initial_setup() -> Result<
	(
		(OnlineClient<Hyperbridge>, OnlineClient<Hyperbridge>),
		(SubstrateClient<Hyperbridge>, SubstrateClient<Hyperbridge>),
		(Arc<dyn IsmpProvider>, Arc<dyn IsmpProvider>),
		(
			Arc<TransactionPayment>,
			RelayerConfig,
			HashMap<StateMachine, Arc<dyn IsmpProvider>>,
			TaskManager,
		),
	),
	anyhow::Error,
> {
	let chain_a_config = SubstrateConfig {
		state_machine: Kusama(2000),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9990".to_string(), // url from local-testnet zombienet config
		max_rpc_payload_size: None,
		signer: Some("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0".to_string()),
		latest_height: None,
		max_concurent_queries: None,
	};

	let chain_b_config = SubstrateConfig {
		state_machine: Kusama(2001),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9991".to_string(),
		max_rpc_payload_size: None,
		signer: Some("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0".to_string()),
		latest_height: None,
		max_concurent_queries: None,
	};

	let (url_a, url_b) = (&chain_a_config.rpc_ws, &chain_b_config.rpc_ws);

	let client_a = subxt_utils::client::ws_client::<Hyperbridge>(url_a.as_str(), u32::MAX).await?;
	let client_b = subxt_utils::client::ws_client::<Hyperbridge>(url_b.as_str(), u32::MAX).await?;

	let relayer_config = RelayerConfig::default();

	// setup state machines
	let chain_a_sub_client =
		SubstrateClient::<KeccakSubstrateChain>::new(chain_a_config.clone()).await?;
	let chain_b_sub_client =
		SubstrateClient::<KeccakSubstrateChain>::new(chain_b_config.clone()).await?;

	let (chain_a_any_config, chain_b_any_config) = (
		AnyConfig::Substrate(chain_a_config.clone()),
		AnyConfig::Substrate(chain_b_config.clone()),
	);
	let chain_a_client = chain_a_any_config
		.clone()
		.into_client(Arc::new(chain_b_sub_client.clone()))
		.await?;
	let chain_b_client = chain_b_any_config
		.clone()
		.into_client(Arc::new(chain_a_sub_client.clone()))
		.await?;

	let mut client_map = HashMap::new();
	client_map.insert(chain_a_config.state_machine, chain_a_client.clone());
	client_map.insert(chain_b_config.state_machine, chain_b_client.clone());

	let tx_payment = Arc::new(
		TransactionPayment::initialize(&"../../../dev.db") // out of hyperbridge directory
			.await
			.map_err(|err| anyhow!("Error initializing database: {err:?}"))?,
	);

	let tokio_handle = tokio::runtime::Handle::current();
	let task_manager = TaskManager::new(tokio_handle, None)?;

	Ok((
		(client_a, client_b),
		(chain_a_sub_client, chain_b_sub_client),
		(chain_a_client, chain_b_client),
		(tx_payment, relayer_config, client_map, task_manager),
	))
}

/// Assertion is on ismp related events, and state changes on source and destination chain
/// Alice in chain A sends 100_000 tokens to Alice in chain B
#[tokio::test(flavor = "multi_thread")]
async fn submit_transfer_function_works() -> Result<(), anyhow::Error> {
	log_setup()?;
	let (
		(client_a, client_b),
		(chain_a_sub_client, chain_b_sub_client),
		(_chain_a_client, chain_b_client),
		(tx_payment, relayer_config, client_map, task_manager),
	) = initial_setup().await?;

	log::info!(
		"🧊integration test for para: {} to para {}: fund transfer",
		chain_a_sub_client.state_machine_id().state_id,
		chain_b_sub_client.state_machine_id().state_id
	);

	// Accounts & keys
	let alice_signer = PairSigner::<Hyperbridge, _>::new(
		Pair::from_string("//Alice", None).expect("Unable to create ALice account"),
	);
	let alice_key = api::storage().system().account(AccountId32(dev::alice().public_key().0));

	// initiate message relaying task
	relay(
		chain_a_sub_client,
		chain_b_client.clone(),
		relayer_config.clone(),
		Kusama(2001),
		tx_payment,
		client_map.clone(),
		&task_manager,
	)
	.await?;
	// time delay is not fixed as it gives time for relayer to initiate and do all necessary setup
	// and starting to fetch events
	tokio::time::sleep(Duration::from_secs(10)).await;

	let amount = 100_000_000000000000;
	let transfer_call = api::tx().ismp_demo().transfer(TransferParams {
		to: AccountId32(dev::alice().public_key().0),
		amount,
		para_id: 2001,
		timeout: 70,
	});

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

	let result = client_a
		.tx()
		.sign_and_submit_then_watch_default(&transfer_call, &alice_signer)
		.await?
		.wait_for_finalized_success()
		.await?
		.all_events_in_block()
		.clone();

	let tx_block_hash = result.block_hash();

	let events = client_a.events().at(tx_block_hash).await?;
	log::info!("Ismp Events: {:?} \n", events.find_last::<Request>()?);

	// Assert burnt & transferred tokens in chain A
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

	tokio::time::sleep(Duration::from_secs(40)).await;

	// The relayer should finish sending the request message to chain B

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

	// diving by 10000000000 for better assertion as the rem balance = initial - amount - fees
	// in chain A
	assert_eq!(
		(alice_chain_a_initial_balance - amount) / 100000000000,
		alice_chain_a_new_balance / 100000000000
	);
	// in chain B
	assert_eq!(
		(alice_chain_b_initial_balance + amount) / 100000000000,
		alice_chain_b_new_balance / 100000000000
	);

	Ok(())
}

/// fetch a foreign storage item from a given key
#[tokio::test(flavor = "multi_thread")]
async fn get_request_works() -> Result<(), anyhow::Error> {
	log_setup()?;
	let (
		(client_a, client_b),
		(chain_a_sub_client, chain_b_sub_client),
		(chain_a_client, chain_b_client),
		(_, relayer_config, client_map, _),
	) = initial_setup().await?;

	log::info!(
		"🧊integration test for para: {} to para {}: get request",
		chain_a_sub_client.state_machine_id().state_id,
		chain_b_sub_client.state_machine_id().state_id
	);

	// Accounts & keys
	let alice_signer = PairSigner::<Hyperbridge, _>::new(
		Pair::from_string("//Alice", None).expect("Unable to create ALice account"),
	);
	let alice_key = api::storage().system().account(AccountId32(dev::alice().public_key().0));
	let encoded_chain_b_id_storage_key =
		"0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";

	// Chain A fetch Alice balance from chain B
	let latest_height_b =
		chain_a_client.query_latest_height(chain_b_client.state_machine_id()).await?;

	let get_request = api::tx().ismp_demo().get_request(GetRequest {
		para_id: 2001,
		height: latest_height_b,
		timeout: 70,
		keys: vec![encoded_chain_b_id_storage_key.as_bytes().to_vec()],
	});
	let tx_result = client_a
		.tx()
		.sign_and_submit_then_watch_default(&get_request, &alice_signer)
		.await?
		.wait_for_finalized_success()
		.await?
		.all_events_in_block()
		.clone();

	let tx_block_hash = tx_result.block_hash();
	let tx_block_height = client_a.blocks().at(tx_block_hash).await?.number() as u64;
	let events = client_a.events().at(tx_block_hash).await?;
	log::info!("Ismp Events: {:?} \n", events.find_last::<Request>()?);

	//  Self relay to chain B
	messaging_relayer_lite(
		client_b,
		chain_a_client,
		chain_b_client,
		chain_a_sub_client,
		tx_block_height,
	)
	.await?;

	Ok(())
}
