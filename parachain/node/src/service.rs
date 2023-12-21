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
use std::{sync::Arc, time::Duration};

use cumulus_client_cli::CollatorOptions;
// Local Runtime Types
use crate::runtime_api::{opaque, BaseHostRuntimeApis};

// Cumulus Imports
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;
use cumulus_client_service::{
    build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
    BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::{relay_chain::CollatorPair, ParaId};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

// Substrate Imports
use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use sc_client_api::Backend;
use sc_consensus::ImportQueue;
use sc_executor::{
    HeapAllocStrategy, NativeElseWasmExecutor, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY,
};
use sc_network::NetworkBlock;
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ConstructRuntimeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Keccak256;
use substrate_prometheus_endpoint::Registry;
// use crate::client::Client;

/// Native executor type.
pub struct GargantuanExecutor;

impl sc_executor::NativeExecutionDispatch for GargantuanExecutor {
    type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        gargantuan_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        gargantuan_runtime::native_version()
    }
}

pub struct MessierExecutor;

impl sc_executor::NativeExecutionDispatch for MessierExecutor {
    type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        messier_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        messier_runtime::native_version()
    }
}

pub type FullClient<Runtime, Executor> =
    TFullClient<opaque::Block, Runtime, NativeElseWasmExecutor<Executor>>;

pub type FullBackend = TFullBackend<opaque::Block>;

type ParachainBlockImport<Runtime, Executor> =
    TParachainBlockImport<opaque::Block, Arc<FullClient<Runtime, Executor>>, FullBackend>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<Runtime, Executor>(
    config: &Configuration,
) -> Result<
    PartialComponents<
        FullClient<Runtime, Executor>,
        FullBackend,
        (),
        sc_consensus::DefaultImportQueue<opaque::Block>,
        sc_transaction_pool::FullPool<opaque::Block, FullClient<Runtime, Executor>>,
        (ParachainBlockImport<Runtime, Executor>, Option<Telemetry>, Option<TelemetryWorkerHandle>),
    >,
    sc_service::Error,
>
where
    Runtime:
        ConstructRuntimeApi<opaque::Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
    Runtime::RuntimeApi: BaseHostRuntimeApis,
    sc_client_api::StateBackendFor<FullBackend, opaque::Block>: sp_api::StateBackend<Keccak256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
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

    let heap_pages = config
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static { extra_pages: h as _ });

    let wasm = WasmExecutor::builder()
        .with_execution_method(config.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.max_runtime_instances)
        .with_runtime_cache_size(config.runtime_cache_size)
        .build();

    let executor = NativeElseWasmExecutor::<Executor>::new_with_wasm_executor(wasm);

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

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
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
        select_chain: (),
        other: (block_import, telemetry, telemetry_worker_handle),
    })
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<Runtime, Executor>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<TaskManager>
where
    Runtime:
        ConstructRuntimeApi<opaque::Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
    Runtime::RuntimeApi: BaseHostRuntimeApis,
    sc_client_api::StateBackendFor<FullBackend, opaque::Block>: sp_api::StateBackend<Keccak256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<Runtime, Executor>(&parachain_config)?;
    let (block_import, mut telemetry, telemetry_worker_handle) = params.other;
    let net_config = sc_network::config::FullNetworkConfiguration::new(&parachain_config.network);

    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;

    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
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

    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
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
                network_provider: network.clone(),
                is_validator: parachain_config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    let rpc_builder = {
        let client = client.clone();
        let backend = backend.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
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
        backend,
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
        match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench) {
            Err(err) if validator => {
                log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
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
    })?;

    if validator {
        start_consensus::<Runtime, Executor>(
            client.clone(),
            block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            sync_service.clone(),
            params.keystore_container.keystore(),
            relay_chain_slot_duration,
            para_id,
            collator_key.expect("Command line arguments do not allow this. qed"),
            overseer_handle,
            announce_block,
        )?;
    }

    start_network.start_network();

    Ok(task_manager)
}

/// Build the import queue for the parachain runtime.
fn build_import_queue<Runtime, Executor>(
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
    sc_client_api::StateBackendFor<FullBackend, opaque::Block>: sp_api::StateBackend<Keccak256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

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
        slot_duration,
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
        telemetry,
    ))
}

fn start_consensus<Runtime, Executor>(
    client: Arc<FullClient<Runtime, Executor>>,
    block_import: ParachainBlockImport<Runtime, Executor>,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<
        sc_transaction_pool::FullPool<opaque::Block, FullClient<Runtime, Executor>>,
    >,
    sync_oracle: Arc<SyncingService<opaque::Block>>,
    keystore: KeystorePtr,
    relay_chain_slot_duration: Duration,
    para_id: ParaId,
    collator_key: CollatorPair,
    overseer_handle: OverseerHandle,
    announce_block: Arc<dyn Fn(opaque::Hash, Option<Vec<u8>>) + Send + Sync>,
) -> Result<(), sc_service::Error>
where
    Runtime:
        ConstructRuntimeApi<opaque::Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
    Runtime::RuntimeApi: BaseHostRuntimeApis,
    sc_client_api::StateBackendFor<FullBackend, opaque::Block>: sp_api::StateBackend<Keccak256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    use cumulus_client_consensus_aura::collators::basic::{
        self as basic_aura, Params as BasicAuraParams,
    };

    // NOTE: because we use Aura here explicitly, we can use `CollatorSybilResistance::Resistant`
    // when starting the network.

    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

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

    let params = BasicAuraParams {
        create_inherent_data_providers: move |_, ()| async move { Ok(()) },
        block_import,
        para_client: client,
        relay_client: relay_chain_interface,
        sync_oracle,
        keystore,
        collator_key,
        para_id,
        overseer_handle,
        slot_duration,
        relay_chain_slot_duration,
        proposer,
        collator_service,
        // Very limited proposal time.
        authoring_duration: Duration::from_millis(500),
        collation_request_receiver: None,
    };

    let fut = basic_aura::run::<
        opaque::Block,
        sp_consensus_aura::sr25519::AuthorityPair,
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
        chain if chain.contains("gargantuan") =>
            start_node_impl::<gargantuan_runtime::RuntimeApi, GargantuanExecutor>(
                parachain_config,
                polkadot_config,
                collator_options,
                para_id,
                hwbench,
            )
            .await,
        chain if chain.contains("messier") =>
            start_node_impl::<messier_runtime::RuntimeApi, MessierExecutor>(
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
