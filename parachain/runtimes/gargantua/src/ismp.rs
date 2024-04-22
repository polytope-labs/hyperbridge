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
    AccountId, Assets, Balance, Balances, Gateway, Ismp, Mmr, ParachainInfo, Runtime, RuntimeEvent,
    Timestamp, EXISTENTIAL_DEPOSIT,
};
use frame_support::{
    pallet_prelude::{ConstU32, Get},
    parameter_types,
    traits::AsEnsureOriginWithArg,
    PalletId,
};
use frame_system::EnsureRoot;
use ismp::{
    error::Error,
    host::StateMachine,
    module::IsmpModule,
    router::{IsmpRouter, Post, Request, Response},
};
use pallet_asset_gateway::TokenGatewayParams;
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_runtime::Percent;

use ismp::router::Timeout;
use ismp_sync_committee::constants::sepolia::Sepolia;
use pallet_ismp::{dispatcher::FeeMetadata, host::Host, primitives::ModuleId};
use sp_std::prelude::*;
use staging_xcm::latest::MultiLocation;

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
    type Coprocessor = Coprocessor;
    type TimeProvider = Timestamp;
    type Router = Router;
    type ConsensusClients = (
        ismp_bsc::BscClient<Host<Runtime>>,
        ismp_sync_committee::SyncCommitteeConsensusClient<Host<Runtime>, Sepolia>,
    );
    type Mmr = Mmr;
    type WeightProvider = ();
}

impl pallet_ismp_demo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type IsmpDispatcher = pallet_ismp::dispatcher::Dispatcher<Runtime>;
}

impl pallet_ismp_relayer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_ismp_host_executive::Config for Runtime {}

impl pallet_call_decompressor::Config for Runtime {
    type MaxCallSize = ConstU32<2>;
}

// todo: set corrrect parameters
parameter_types! {
    pub const AssetPalletId: PalletId = PalletId(*b"asset-tx");
    pub const ProtocolAccount: PalletId = PalletId(*b"protocol");
    pub const TransferParams: TokenGatewayParams = TokenGatewayParams::from_parts(Percent::from_percent(1), H160::zero(), H256::zero());
}

impl pallet_asset_gateway::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = AssetPalletId;
    type ProtocolAccount = ProtocolAccount;
    type Params = TransferParams;
    type Assets = Assets;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<MultiLocation> for XcmBenchmarkHelper {
    fn create_asset_id_parameter(id: u32) -> MultiLocation {
        use staging_xcm::v3::Junction::Parachain;
        MultiLocation::new(1, Parachain(id))
    }
}

parameter_types! {
    pub const AssetDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const AssetAccountDeposit: Balance = EXISTENTIAL_DEPOSIT * 2;
    pub const MetadataDepositBase: Balance = EXISTENTIAL_DEPOSIT * 2;
    pub const MetadataDepositPerByte: Balance = EXISTENTIAL_DEPOSIT / 2;
    pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT * 2;
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = MultiLocation;
    type AssetIdParameter = MultiLocation;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId32>>;
    type ForceOrigin = EnsureRoot<AccountId32>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type WeightInfo = ();
    type CallbackHandle = ();
    type Extra = ();
    type RemoveItemsLimit = ConstU32<5>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = XcmBenchmarkHelper;
}

impl IsmpModule for ProxyModule {
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        if request.dest != HostStateMachine::get() {
            let meta = FeeMetadata { origin: [0u8; 32].into(), fee: Default::default() };
            return Ismp::dispatch_request(Request::Post(request), meta);
        }

        let pallet_id = ModuleId::from_bytes(&request.to)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;

        let token_gateway = ModuleId::Evm(Gateway::token_gateway_address());

        match pallet_id {
            pallet_ismp_demo::PALLET_ID =>
                pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_accept(request),
            id if id == token_gateway =>
                pallet_asset_gateway::Module::<Runtime>::default().on_accept(request),
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

        let token_gateway = ModuleId::Evm(Gateway::token_gateway_address());
        match pallet_id {
            pallet_ismp_demo::PALLET_ID =>
                pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_response(response),
            id if id == token_gateway =>
                pallet_asset_gateway::Module::<Runtime>::default().on_response(response),
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
        let token_gateway = ModuleId::Evm(Gateway::token_gateway_address());
        match pallet_id {
            pallet_ismp_demo::PALLET_ID =>
                pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_timeout(timeout),
            id if id == token_gateway =>
                pallet_asset_gateway::Module::<Runtime>::default().on_timeout(timeout),
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
