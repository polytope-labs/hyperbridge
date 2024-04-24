use crate::runtime_api::{opaque, BaseHostRuntimeApis};
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_simnode::{parachain::ParachainSelectChain, Executor};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_runtime::traits::Keccak256;
use std::sync::Arc;

type FullClient<Runtime> = TFullClient<opaque::Block, Runtime, sc_simnode::Executor>;

type FullBackend = TFullBackend<opaque::Block>;

type ParachainBlockImport<Runtime> =
    TParachainBlockImport<opaque::Block, Arc<FullClient<Runtime>>, FullBackend>;

pub struct GargantuaRuntimeInfo;

impl sc_simnode::ChainInfo for GargantuaRuntimeInfo {
    // make sure you pass the opaque::Block here

    type Block = gargantua_runtime::opaque::Block;
    // the runtime type
    type Runtime = gargantua_runtime::Runtime;
    // the runtime api
    type RuntimeApi = gargantua_runtime::RuntimeApi;
    // [`SignedExtra`] for your runtime
    type SignedExtras = gargantua_runtime::SignedExtra;

    // initialize the [`SignedExtra`] for your runtime, you'll notice I'm calling a pallet method in
    // order to read from storage. This is possible becase this method is called in an externalities
    // provided environment. So feel free to reasd your runtime storage.
    fn signed_extras(
        from: <Self::Runtime as frame_system::pallet::Config>::AccountId,
    ) -> Self::SignedExtras {
        use sp_runtime::generic::Era;
        let nonce = frame_system::Pallet::<Self::Runtime>::account_nonce(from);
        (
            frame_system::CheckNonZeroSender::<Self::Runtime>::new(),
            frame_system::CheckSpecVersion::<Self::Runtime>::new(),
            frame_system::CheckTxVersion::<Self::Runtime>::new(),
            frame_system::CheckGenesis::<Self::Runtime>::new(),
            frame_system::CheckEra::<Self::Runtime>::from(Era::Immortal),
            frame_system::CheckNonce::<Self::Runtime>::from(nonce),
            frame_system::CheckWeight::<Self::Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Self::Runtime>::from(0),
        )
    }
}

/// Build the import queue for the parachain runtime.
pub(crate) fn build_import_queue<Runtime>(
    client: Arc<FullClient<Runtime>>,
    block_import: ParachainBlockImport<Runtime>,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<opaque::Block>, sc_service::Error>
where
    Runtime: ConstructRuntimeApi<opaque::Block, FullClient<Runtime>> + Send + Sync + 'static,
    Runtime::RuntimeApi: BaseHostRuntimeApis,
    sc_client_api::StateBackendFor<FullBackend, opaque::Block>:
        sc_client_api::StateBackend<Keccak256>,
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

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial_with_executor<Runtime>(
    config: &Configuration,
    executor: Executor,
) -> Result<
    PartialComponents<
        FullClient<Runtime>,
        FullBackend,
        ParachainSelectChain<FullClient<Runtime>>,
        sc_consensus::DefaultImportQueue<opaque::Block>,
        sc_transaction_pool::FullPool<opaque::Block, FullClient<Runtime>>,
        (ParachainBlockImport<Runtime>, Option<Telemetry>, Option<TelemetryWorkerHandle>),
    >,
    sc_service::Error,
>
where
    Runtime: ConstructRuntimeApi<opaque::Block, FullClient<Runtime>> + Send + Sync + 'static,
    Runtime::RuntimeApi: BaseHostRuntimeApis,
    sc_client_api::StateBackendFor<FullBackend, opaque::Block>:
        sc_client_api::StateBackend<Keccak256>,
{
    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<opaque::Block, Runtime, _>(config, None, executor)?;
    let client = Arc::new(client);

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

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let block_import = ParachainBlockImport::<Runtime>::new(client.clone(), backend.clone());

    let import_queue =
        build_import_queue(client.clone(), block_import.clone(), config, None, &task_manager)?;

    let select_chain = ParachainSelectChain::new(client.clone());

    Ok(PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain,
        other: (block_import, None, None),
    })
}
