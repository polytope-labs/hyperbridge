// Copyright (C) Polytope Labs Ltd.
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

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

// std
use polkadot_sdk::*;
use std::{sync::Arc, time::Duration};

use cumulus_client_cli::CollatorOptions;
// Local Runtime Types
use crate::runtime_api::{opaque, BaseHostRuntimeApis};

// Cumulus Imports
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_aura::collators::lookahead;
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;
use cumulus_client_service::{
	build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
	BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::{relay_chain::CollatorPair, ParaId};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};
use polkadot_primitives::ValidationCode;
// Substrate Imports
use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use sc_client_api::Backend;
use sc_consensus::ImportQueue;
use sc_executor::{RuntimeVersionOf, WasmExecutor};
use sc_network::{NetworkBackend, NetworkBlock};
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_simnode::parachain::ParachainSelectChain;
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ConstructRuntimeApi;
use sp_core::traits::CodeExecutor;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Keccak256;
use substrate_prometheus_endpoint::Registry;

#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = cumulus_client_service::ParachainHostFunctions;

#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	cumulus_client_service::ParachainHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
);

pub type FullClient<Runtime, Executor = WasmExecutor<HostFunctions>> =
	TFullClient<opaque::Block, Runtime, Executor>;

pub type FullBackend = TFullBackend<opaque::Block>;

type ParachainBlockImport<Runtime, Executor = WasmExecutor<HostFunctions>> =
	TParachainBlockImport<opaque::Block, Arc<FullClient<Runtime, Executor>>, FullBackend>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<Runtime, Executor>(
	config: &Configuration,
	executor: Executor,
) -> Result<
	PartialComponents<
		FullClient<Runtime, Executor>,
		FullBackend,
		ParachainSelectChain<FullClient<Runtime, Executor>>,
		sc_consensus::DefaultImportQueue<opaque::Block>,
		sc_transaction_pool::TransactionPoolHandle<opaque::Block, FullClient<Runtime, Executor>>,
		(ParachainBlockImport<Runtime, Executor>, Option<Telemetry>, Option<TelemetryWorkerHandle>),
	>,
	sc_service::Error,
>
where
	Runtime:
		ConstructRuntimeApi<opaque::Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
	Runtime::RuntimeApi: BaseHostRuntimeApis,
	sc_client_api::StateBackendFor<FullBackend, opaque::Block>:
		sc_client_api::StateBackend<Keccak256>,
	Executor: CodeExecutor + RuntimeVersionOf + 'static,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<opaque::Block, Runtime, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});
	let select_chain = ParachainSelectChain::new(client.clone());

	// Spawn mmr canonicalizing task
	task_manager.spawn_handle().spawn(
		"mmr-canonicalizing-gadget",
		"mmr-gadget",
		mmr_gadget::MmrGadget::start(
			client.clone(),
			backend.clone(),
			sp_mmr_primitives::INDEXING_PREFIX.to_vec(),
		),
	);

	let transaction_pool = Arc::from(
		sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);

	let block_import =
		ParachainBlockImport::<Runtime, Executor>::new(client.clone(), backend.clone());

	let import_queue = build_import_queue(
		client.clone(),
		block_import.clone(),
		config,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
	)?;

	Ok(PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (block_import, telemetry, telemetry_worker_handle),
	})
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<Runtime>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<TaskManager>
where
	Runtime: ConstructRuntimeApi<opaque::Block, FullClient<Runtime>> + Send + Sync + 'static,
	Runtime::RuntimeApi: BaseHostRuntimeApis,
	sc_client_api::StateBackendFor<FullBackend, opaque::Block>:
		sc_client_api::StateBackend<Keccak256>,
{
	let parachain_config = prepare_node_config(parachain_config);
	let executor = sc_service::new_wasm_executor::<HostFunctions>(&parachain_config.executor);
	let params = new_partial::<Runtime, _>(&parachain_config, executor)?;
	let (block_import, mut telemetry, telemetry_worker_handle) = params.other;
	let net_config =
		sc_network::config::FullNetworkConfiguration::<
			_,
			_,
			sc_network::NetworkWorker<opaque::Block, opaque::Hash>,
		>::new(&parachain_config.network, parachain_config.prometheus_registry().cloned());

	let client = params.client.clone();
	let backend = params.backend.clone();
	let mut task_manager = params.task_manager;

	let (relay_chain_interface, collator_key, _relay_chain_network, _paranode_rx) =
		build_relay_chain_interface(
			polkadot_config,
			&parachain_config,
			telemetry_worker_handle,
			&mut task_manager,
			collator_options.clone(),
			hwbench.clone(),
		)
		.await
		.map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

	let validator = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue_service = params.import_queue.service();

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		build_network(BuildNetworkParams {
			parachain_config: &parachain_config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			para_id,
			spawn_handle: task_manager.spawn_handle(),
			relay_chain_interface: relay_chain_interface.clone(),
			import_queue: params.import_queue,
			sybil_resistance_level: CollatorSybilResistance::Resistant, // because of Aura
			metrics: sc_network::NetworkWorker::<opaque::Block, opaque::Hash>::register_notification_metrics(
				parachain_config.prometheus_config.as_ref().map(|config| &config.registry),
			),
		})
		.await?;

	if parachain_config.offchain_worker.enabled {
		use futures::FutureExt;

		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(params.keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				is_validator: parachain_config.role.is_authority(),
				enable_http_requests: false,
				custom_extensions: move |_| vec![],
			})?
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	let rpc_builder = {
		let client = client.clone();
		let backend = backend.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |_| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				backend: backend.clone(),
			};

			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.keystore(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);
		// Here you can check whether the hardware meets your chains' requirements. Putting a link
		// in there and swapping out the requirements for your own are probably a good idea. The
		// requirements for a para-chain are dictated by its relay-chain.
		// match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench) {
		//     Err(err) if validator => {
		//         log::warn!(
		// 		"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
		// 		err
		// 	);
		//     },
		//     _ => {},
		// }

		match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench, validator) {
			Err(err) if validator => {
				log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority' find out more at:\n\
				https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot#reference-hardware",
				err
			);
			},
			_ => {},
		}

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);

	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| sc_service::Error::Application(Box::new(e)))?;

	start_relay_chain_tasks(StartRelayChainTasksParams {
		client: client.clone(),
		announce_block: announce_block.clone(),
		para_id,
		relay_chain_interface: relay_chain_interface.clone(),
		task_manager: &mut task_manager,
		da_recovery_profile: if validator {
			DARecoveryProfile::Collator
		} else {
			DARecoveryProfile::FullNode
		},
		import_queue: import_queue_service,
		relay_chain_slot_duration,
		recovery_handle: Box::new(overseer_handle.clone()),
		sync_service: sync_service.clone(),
		prometheus_registry: prometheus_registry.as_ref(),
	})?;

	if validator {
		start_consensus::<Runtime>(
			client.clone(),
			backend,
			block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface.clone(),
			transaction_pool,
			params.keystore_container.keystore(),
			relay_chain_slot_duration,
			para_id,
			collator_key.expect("Command line arguments do not allow this. qed"),
			overseer_handle,
			announce_block,
		)?;
	}

	Ok(task_manager)
}

/// Build the import queue for the parachain runtime.
pub(crate) fn build_import_queue<Runtime, Executor>(
	client: Arc<FullClient<Runtime, Executor>>,
	block_import: ParachainBlockImport<Runtime, Executor>,
	config: &Configuration,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<opaque::Block>, sc_service::Error>
where
	Runtime:
		ConstructRuntimeApi<opaque::Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
	Runtime::RuntimeApi: BaseHostRuntimeApis,
	sc_client_api::StateBackendFor<FullBackend, opaque::Block>:
		sc_client_api::StateBackend<Keccak256>,
	Executor: CodeExecutor + RuntimeVersionOf + 'static,
{
	Ok(cumulus_client_consensus_aura::equivocation_import_queue::fully_verifying_import_queue::<
		sp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
	>(
		client,
		block_import,
		move |_, _| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			Ok(timestamp)
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		telemetry,
	))
}

fn start_consensus<Runtime>(
	client: Arc<FullClient<Runtime>>,
	backend: Arc<FullBackend>,
	block_import: ParachainBlockImport<Runtime>,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	transaction_pool: Arc<
		sc_transaction_pool::TransactionPoolHandle<opaque::Block, FullClient<Runtime>>,
	>,
	keystore: KeystorePtr,
	relay_chain_slot_duration: Duration,
	para_id: ParaId,
	collator_key: CollatorPair,
	overseer_handle: OverseerHandle,
	announce_block: Arc<dyn Fn(opaque::Hash, Option<Vec<u8>>) + Send + Sync>,
) -> Result<(), sc_service::Error>
where
	Runtime: ConstructRuntimeApi<opaque::Block, FullClient<Runtime>> + Send + Sync + 'static,
	Runtime::RuntimeApi: BaseHostRuntimeApis,
	sc_client_api::StateBackendFor<FullBackend, opaque::Block>:
		sc_client_api::StateBackend<Keccak256>,
{
	// NOTE: because we use Aura here explicitly, we can use `CollatorSybilResistance::Resistant`
	// when starting the network.
	let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool,
		prometheus_registry,
		telemetry.clone(),
	);

	let proposer = Proposer::new(proposer_factory);

	let collator_service = CollatorService::new(
		client.clone(),
		Arc::new(task_manager.spawn_handle()),
		announce_block,
		client.clone(),
	);

	let (client_clone, relay_chain_interface_clone) =
		(client.clone(), relay_chain_interface.clone());
	let params = lookahead::Params {
		create_inherent_data_providers: move |parent, ()| {
			let client = client_clone.clone();
			let relay_chain_interface = relay_chain_interface_clone.clone();
			async move {
				let inherent = ismp_parachain_inherent::ConsensusInherentProvider::create(
					parent,
					client,
					relay_chain_interface,
				)
				.await?;

				Ok(inherent)
			}
		},
		block_import,
		para_client: client.clone(),
		para_backend: backend,
		relay_client: relay_chain_interface,
		code_hash_provider: move |hash| {
			client.code_at(hash).ok().map(ValidationCode).map(|c| c.hash())
		},
		keystore,
		collator_key,
		para_id,
		overseer_handle,
		reinitialize: true,
		relay_chain_slot_duration,
		proposer,
		collator_service,
		// Async backing time
		authoring_duration: Duration::from_millis(1500),
		max_pov_percentage: None,
	};

	let fut = lookahead::run::<
		opaque::Block,
		sp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
		_,
		_,
		_,
		_,
	>(params);
	task_manager.spawn_essential_handle().spawn("aura", None, fut);

	Ok(())
}

/// Start a parachain node.
pub async fn start_parachain_node(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<TaskManager> {
	match parachain_config.chain_spec.id() {
		chain if chain.contains("gargantua") =>
			start_node_impl::<gargantua_runtime::RuntimeApi>(
				parachain_config,
				polkadot_config,
				collator_options,
				para_id,
				hwbench,
			)
			.await,
		chain if chain.contains("nexus") =>
			start_node_impl::<nexus_runtime::RuntimeApi>(
				parachain_config,
				polkadot_config,
				collator_options,
				para_id,
				hwbench,
			)
			.await,
		chain => panic!("Unknown chain with id: {}", chain),
	}
}
