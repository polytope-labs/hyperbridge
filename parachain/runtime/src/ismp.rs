use crate::{
    alloc::{boxed::Box, string::ToString},
    AccountId, Balance, Balances, Ismp, ParachainInfo, Runtime, RuntimeEvent,
    Timestamp,
};
use alloc::format;
use frame_support::pallet_prelude::Get;
use frame_system::EnsureRoot;
use ismp::{
    consensus::{ConsensusClient, ConsensusClientId},
    error::Error,
    host::StateMachine,
    module::IsmpModule,
    router::{IsmpRouter, Post, Request, Response},
};

use pallet_ismp::{
    host::Host,
    primitives::{ConsensusClientProvider, ModuleId},
};
use sp_std::prelude::*;

#[derive(Default)]
pub struct ProxyModule;

pub struct StateMachineProvider;

impl Get<StateMachine> for StateMachineProvider {
    fn get() -> StateMachine {
        StateMachine::Kusama(ParachainInfo::get().into())
    }
}

pub struct ConsensusProvider;

impl ConsensusClientProvider for ConsensusProvider {
    fn consensus_client(id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        match id {
            // ismp_parachain::PARACHAIN_CONSENSUS_ID => {
            //     let parachain =
            //         ismp_parachain::ParachainConsensusClient::<Runtime, IsmpParachain>::default();
            //     Ok(Box::new(parachain))
            // },
            ismp_sync_committee::BEACON_CONSENSUS_ID => {
                let sync_committee =
                    ismp_sync_committee::SyncCommitteeConsensusClient::<Host<Runtime>>::default();
                Ok(Box::new(sync_committee))
            },
            id => Err(Error::ImplementationSpecific(format!("Unknown consensus client: {id:?}")))?,
        }
    }
}

// impl ismp_parachain::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
// }

impl pallet_ismp::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<AccountId>;
    type StateMachine = StateMachineProvider;
    type TimeProvider = Timestamp;
    type IsmpRouter = Router;
    type ConsensusClientProvider = ConsensusProvider;
    type WeightInfo = ();
    type WeightProvider = ();
}

impl ismp_demo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type IsmpDispatcher = pallet_ismp::dispatcher::Dispatcher<Runtime>;
}

impl IsmpModule for ProxyModule {
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        if request.dest != StateMachineProvider::get() {
            return Ismp::dispatch_request(Request::Post(request))
        }

        let pallet_id = ModuleId::from_bytes(&request.to)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_accept(request),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
        }
    }

    fn on_response(&self, response: Response) -> Result<(), Error> {
        if response.dest_chain() != StateMachineProvider::get() {
            return Ismp::dispatch_response(response)
        }

        let request = &response.request();
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = ModuleId::from_bytes(from)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_response(response),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
        }
    }

    fn on_timeout(&self, request: Request) -> Result<(), Error> {
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = ModuleId::from_bytes(from)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_timeout(request),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
        }
    }
}

#[derive(Default)]
pub struct Router;

impl IsmpRouter for Router {
    fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        Ok(Box::new(ProxyModule::default()))
    }
}
