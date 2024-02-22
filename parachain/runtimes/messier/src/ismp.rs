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

use crate::{
    alloc::{boxed::Box, string::ToString},
    AccountId, Balance, Balances, Ismp, ParachainInfo, Runtime, RuntimeEvent, Timestamp,
};

use frame_support::pallet_prelude::Get;
use frame_system::EnsureRoot;
use ismp::{
    error::Error,
    host::StateMachine,
    module::IsmpModule,
    router::{IsmpRouter, Post, Request, Response},
};

use ismp::router::Timeout;

use ismp_sync_committee::constants::sepolia::Sepolia;
use pallet_ismp::{dispatcher::FeeMetadata, host::Host, primitives::ModuleId};
use sp_std::prelude::*;

#[derive(Default)]
pub struct ProxyModule;

pub struct HostStateMachine;

impl Get<StateMachine> for HostStateMachine {
    fn get() -> StateMachine {
        StateMachine::Kusama(ParachainInfo::get().into())
    }
}

impl ismp_sync_committee::pallet::Config for Runtime {
    type AdminOrigin = EnsureRoot<AccountId>;
}

pub struct Coprocessor;

impl Get<Option<StateMachine>> for Coprocessor {
    fn get() -> Option<StateMachine> {
        Some(HostStateMachine::get())
    }
}

impl pallet_ismp::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<AccountId>;
    type HostStateMachine = HostStateMachine;
    type TimeProvider = Timestamp;
    type Router = Router;
    type Coprocessor = Coprocessor;
    type ConsensusClients = (
        ismp_bsc_pos::BscClient<Host<Runtime>>,
        ismp_sync_committee::SyncCommitteeConsensusClient<Host<Runtime>, Sepolia>,
    );
    type WeightInfo = ();
    type WeightProvider = ();
}

impl ismp_demo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type IsmpDispatcher = pallet_ismp::dispatcher::Dispatcher<Runtime>;
}

impl ismp_polygon_pos::pallet::Config for Runtime {}

impl IsmpModule for ProxyModule {
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        if request.dest != HostStateMachine::get() {
            let meta = FeeMetadata { origin: [0u8; 32].into(), fee: Default::default() };
            return Ismp::dispatch_request(Request::Post(request), meta);
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
        if response.dest_chain() != HostStateMachine::get() {
            let meta = FeeMetadata { origin: [0u8; 32].into(), fee: Default::default() };
            return Ismp::dispatch_response(response, meta);
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

    fn on_timeout(&self, timeout: Timeout) -> Result<(), Error> {
        let from = match &timeout {
            Timeout::Request(Request::Post(post)) => &post.from,
            Timeout::Request(Request::Get(get)) => &get.from,
            Timeout::Response(res) => &res.post.to,
        };

        let pallet_id = ModuleId::from_bytes(from)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_timeout(timeout),
            // instead of returning an error, do nothing. The timeout is for a connected chain.
            _ => Ok(()),
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
