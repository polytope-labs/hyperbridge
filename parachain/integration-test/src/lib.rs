#![cfg(test)]
#![deny(missing_docs, unused_imports)]

use anyhow::anyhow;
use ismp::host::{StateMachine, StateMachine::Kusama};
use sc_service::TaskManager;
use std::{collections::HashMap, sync::Arc, time::Duration};
use subxt::{
	ext::sp_core::{sr25519::Pair, Pair as PairT},
	tx::PairSigner,
	utils::AccountId32,
	OnlineClient,
};
use subxt_signer::sr25519::dev::{self};
use subxt_utils::{
	gargantua::{
		api,
		api::{ismp::events::Request, runtime_types::pallet_ismp_demo::pallet::TransferParams},
	},
	Hyperbridge,
};
use tesseract::logging::setup as log_setup;
use tesseract_config::AnyConfig;
use tesseract_messaging::relay;
use tesseract_primitives::{config::RelayerConfig, IsmpProvider};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient, SubstrateConfig};
use transaction_fees::TransactionPayment;

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
	let chain_a_config = AnyConfig::Substrate(SubstrateConfig {
		state_machine: Kusama(2000),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9990".to_string(), // url from local-testnet zombienet config
		max_rpc_payload_size: None,
		signer: Some("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0".to_string()),
		latest_height: None,
		max_concurent_queries: None,
	});

	let chain_b_config = AnyConfig::Substrate(SubstrateConfig {
		state_machine: Kusama(2001),
		hashing: None,
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://127.0.0.1:9991".to_string(),
		max_rpc_payload_size: None,
		signer: Some("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0".to_string()),
		latest_height: None,
		max_concurent_queries: None,
	});

	let (url_a, chain_a_substrate_config, url_b, chain_b_substrate_config) =
		match (chain_a_config.clone(), chain_b_config.clone()) {
			(
				AnyConfig::Substrate(substrate_config_a),
				AnyConfig::Substrate(substrate_config_b),
			) => (
				substrate_config_a.clone().rpc_ws,
				substrate_config_a,
				substrate_config_b.clone().rpc_ws,
				substrate_config_b,
			),
			(AnyConfig::Evm(_), AnyConfig::Evm(_)) => {
				todo!("Implement EVM chain")
			},
			_ => {
				todo!("Implement EVM chain")
			},
		};

	let (client_a, client_b) = {
		let client_a =
			subxt_utils::client::ws_client::<Hyperbridge>(url_a.as_str(), u32::MAX).await?;
		let client_b =
			subxt_utils::client::ws_client::<Hyperbridge>(url_b.as_str(), u32::MAX).await?;
		(client_a, client_b)
	};

	let relayer_config = RelayerConfig {
		module_filter: None,
		delivery_endpoints: vec![Kusama(2000), Kusama(2001)],
		minimum_profit_percentage: 0,
		withdrawal_frequency: None,
		minimum_withdrawal_amount: None,
		unprofitable_retry_frequency: None,
		deliver_failed: None,
	};

	// setup state machines
	let chain_a_sub_client =
		SubstrateClient::<KeccakSubstrateChain>::new(chain_a_substrate_config.clone()).await?;
	let chain_b_sub_client =
		SubstrateClient::<KeccakSubstrateChain>::new(chain_b_substrate_config.clone()).await?;

	let chain_a_client =
		chain_a_config.clone().into_client(Arc::new(chain_b_sub_client.clone())).await?;
	let chain_b_client =
		chain_b_config.clone().into_client(Arc::new(chain_a_sub_client.clone())).await?;

	let mut client_map = HashMap::new();
	client_map.insert(chain_a_substrate_config.state_machine, chain_a_client.clone());
	client_map.insert(chain_b_substrate_config.state_machine, chain_b_client.clone());

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

	let amount = 100_000;
	let transfer_call = api::tx().ismp_demo().transfer(TransferParams {
		to: AccountId32(dev::alice().public_key().0),
		amount,
		para_id: 2001,
		timeout: 70,
	});

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

	let alice_signer = PairSigner::<Hyperbridge, _>::new(
		Pair::from_string("//Alice", None).expect("Unable to create ALice account"),
	);

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
		(alice_chain_a_initial_balance - amount) / 10000000000,
		alice_chain_a_new_balance / 10000000000
	);
	// in chain B
	assert_eq!(
		(alice_chain_b_initial_balance + amount) / 10000000000,
		alice_chain_b_new_balance / 10000000000
	);

	Ok(())
}

/// fetch a foreign storage item from a given key
#[tokio::test(flavor = "multi_thread")]
async fn get_request_works() -> Result<(), anyhow::Error> {
	Ok(())
}
