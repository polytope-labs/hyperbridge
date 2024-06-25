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
use ismp_sync_committee::constants::mainnet::Mainnet;
use pallet_ismp::{dispatcher::FeeMetadata, ModuleId};
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
	type IsmpHost = Ismp;
}

pub struct Coprocessor;

impl Get<Option<StateMachine>> for Coprocessor {
	fn get() -> Option<StateMachine> {
		Some(HostStateMachine::get())
	}
}

impl pallet_ismp::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = EnsureRoot<AccountId>;
	type HostStateMachine = HostStateMachine;
	type TimestampProvider = Timestamp;
	type Router = Router;
	type Balance = Balance;
	type Currency = Balances;
	type Coprocessor = Coprocessor;
	type ConsensusClients = (
		ismp_bsc::BscClient<Ismp>,
		ismp_sync_committee::SyncCommitteeConsensusClient<Ismp, Mainnet>,
	);
	type Mmr = Mmr;
	type WeightProvider = ();
}

impl pallet_ismp_relayer::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

impl pallet_ismp_host_executive::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

impl ismp_parachain::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

impl pallet_call_decompressor::Config for Runtime {
	type MaxCallSize = ConstU32<3>;
}

// todo: set corrrect Token Gateway parameters
parameter_types! {
	pub const AssetPalletId: PalletId = PalletId(*b"asset-tx");
	pub const ProtocolAccount: PalletId = PalletId(*b"protocol");
	pub const TransferParams: TokenGatewayParams = TokenGatewayParams::from_parts(Permill::from_parts(1_000)); // 0.1%
}

impl pallet_asset_gateway::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = AssetPalletId;
	type ProtocolAccount = ProtocolAccount;
	type Params = TransferParams;
	type IsmpHost = Ismp;
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
			let meta = FeeMetadata::<Runtime> { payer: [0u8; 32].into(), fee: Default::default() };
			Ismp::dispatch_request(Request::Post(request), meta)?;
			return Ok(());
		}

		let pallet_id =
			ModuleId::from_bytes(&request.to).map_err(|err| Error::Custom(err.to_string()))?;

		let token_gateway = ModuleId::Evm(Gateway::token_gateway_address());

		match pallet_id {
			id if id == token_gateway =>
				pallet_asset_gateway::Module::<Runtime>::default().on_accept(request),
			_ => Err(Error::Custom("Destination module not found".to_string())),
		}
	}

	fn on_response(&self, response: Response) -> Result<(), Error> {
		if response.dest_chain() != HostStateMachine::get() {
			let meta = FeeMetadata::<Runtime> { payer: [0u8; 32].into(), fee: Default::default() };
			Ismp::dispatch_response(response, meta)?;
			return Ok(());
		}

		let request = &response.request();
		let from = match &request {
			Request::Post(post) => &post.from,
			Request::Get(get) => &get.from,
		};

		let pallet_id = ModuleId::from_bytes(from).map_err(|err| Error::Custom(err.to_string()))?;

		let token_gateway = ModuleId::Evm(Gateway::token_gateway_address());
		match pallet_id {
			id if id == token_gateway =>
				pallet_asset_gateway::Module::<Runtime>::default().on_response(response),
			_ => Err(Error::Custom("Destination module not found".to_string())),
		}
	}

	fn on_timeout(&self, timeout: Timeout) -> Result<(), Error> {
		let from = match &timeout {
			Timeout::Request(Request::Post(post)) => &post.from,
			Timeout::Request(Request::Get(get)) => &get.from,
			Timeout::Response(res) => &res.post.to,
		};

		let pallet_id = ModuleId::from_bytes(from).map_err(|err| Error::Custom(err.to_string()))?;
		let token_gateway = ModuleId::Evm(Gateway::token_gateway_address());
		match pallet_id {
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
