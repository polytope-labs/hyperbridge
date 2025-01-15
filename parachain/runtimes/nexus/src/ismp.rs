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
	weights, AccountId, Assets, Balance, Balances, Ismp, IsmpParachain, Mmr, ParachainInfo,
	Runtime, RuntimeEvent, Timestamp, TokenGatewayInspector, TokenGovernor, TreasuryPalletId,
	XcmGateway, EXISTENTIAL_DEPOSIT,
};
use frame_support::{
	pallet_prelude::{ConstU32, Get},
	parameter_types,
	traits::AsEnsureOriginWithArg,
	PalletId,
};
use frame_system::EnsureRoot;
use hyperbridge_client_machine::HyperbridgeClientMachine;
use ismp::{
	error::Error,
	host::StateMachine,
	module::IsmpModule,
	router::{IsmpRouter, PostRequest, Request, Response},
};
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use pallet_xcm_gateway::AssetGatewayParams;
use sp_core::{crypto::AccountId32, H256};

use anyhow::anyhow;
use ismp::router::Timeout;
use ismp_sync_committee::constants::{gnosis, mainnet::Mainnet};
use pallet_ismp::{dispatcher::FeeMetadata, ModuleId};
use sp_runtime::Permill;
use sp_std::prelude::*;
#[cfg(feature = "runtime-benchmarks")]
use staging_xcm::latest::Location;

#[derive(Default)]
pub struct ProxyModule;

pub struct HostStateMachine;

impl Get<StateMachine> for HostStateMachine {
	fn get() -> StateMachine {
		StateMachine::Polkadot(ParachainInfo::get().into())
	}
}

pub type Ethereum = ismp_sync_committee::pallet::Instance1;
pub type Gnosis = ismp_sync_committee::pallet::Instance2;

impl ismp_sync_committee::pallet::Config<Ethereum> for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
}

impl ismp_sync_committee::pallet::Config<Gnosis> for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
}

impl pallet_state_coprocessor::Config for Runtime {
	type IsmpHost = Ismp;
	type Mmr = Mmr;
}

pub struct Coprocessor;

impl Get<Option<StateMachine>> for Coprocessor {
	fn get() -> Option<StateMachine> {
		Some(HostStateMachine::get())
	}
}

impl ismp_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = pallet_ismp::Pallet<Runtime>;
	type WeightInfo = weights::ismp_grandpa::WeightInfo<Runtime>;
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
		ismp_bsc::BscClient<Ismp, Runtime, ismp_bsc::Mainnet>,
		ismp_sync_committee::SyncCommitteeConsensusClient<Ismp, Mainnet, Runtime, Ethereum>,
		ismp_sync_committee::SyncCommitteeConsensusClient<Ismp, gnosis::Mainnet, Runtime, Gnosis>,
		ismp_parachain::ParachainConsensusClient<
			Runtime,
			IsmpParachain,
			HyperbridgeClientMachine<Runtime, Ismp>,
		>,
		ismp_grandpa::consensus::GrandpaConsensusClient<
			Runtime,
			HyperbridgeClientMachine<Runtime, Ismp>,
		>,
	);
	type OffchainDB = Mmr;
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

impl pallet_call_decompressor::Config for Runtime {
	type MaxCallSize = ConstU32<3>;
}

impl pallet_fishermen::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

impl ismp_parachain::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
	type WeightInfo = weights::ismp_parachain::WeightInfo<Runtime>;
}

parameter_types! {
	pub const AssetPalletId: PalletId = PalletId(*b"asset-tx");
	pub const TransferParams: AssetGatewayParams = AssetGatewayParams::from_parts(Permill::from_parts(1_000)); // 0.1%
}

impl pallet_xcm_gateway::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = AssetPalletId;
	type Params = TransferParams;
	type IsmpHost = Ismp;
	type Assets = Assets;
}

impl pallet_token_governor::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Dispatcher = Ismp;
	type TreasuryAccount = TreasuryPalletId;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<H256> for XcmBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> H256 {
		use codec::Encode;
		use staging_xcm::v4::Junction::Parachain;
		sp_io::hashing::keccak_256(&Location::new(1, Parachain(id)).encode()).into()
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
	type AssetId = H256;
	type AssetIdParameter = H256;
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
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
	type CallbackHandle = ();
	type Extra = ();
	type RemoveItemsLimit = ConstU32<5>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = XcmBenchmarkHelper;
}

impl pallet_token_gateway_inspector::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl IsmpModule for ProxyModule {
	fn on_accept(&self, request: PostRequest) -> Result<(), anyhow::Error> {
		if request.dest != HostStateMachine::get() {
			TokenGatewayInspector::inspect_request(&request)?;

			Ismp::dispatch_request(
				Request::Post(request),
				FeeMetadata::<Runtime> { payer: [0u8; 32].into(), fee: Default::default() },
			)?;
			return Ok(());
		}

		let pallet_id =
			ModuleId::from_bytes(&request.to).map_err(|err| Error::Custom(err.to_string()))?;

		let xcm_gateway = ModuleId::Evm(XcmGateway::token_gateway_address(&request.source));
		let token_governor = ModuleId::Pallet(PalletId(pallet_token_governor::PALLET_ID));

		match pallet_id {
			id if id == xcm_gateway =>
				pallet_xcm_gateway::Module::<Runtime>::default().on_accept(request),
			id if id == token_governor => TokenGovernor::default().on_accept(request),
			_ => Err(anyhow!("Destination module not found")),
		}
	}

	fn on_response(&self, response: Response) -> Result<(), anyhow::Error> {
		if response.dest_chain() != HostStateMachine::get() {
			Ismp::dispatch_response(
				response,
				FeeMetadata::<Runtime> { payer: [0u8; 32].into(), fee: Default::default() },
			)?;
			return Ok(());
		}

		Err(anyhow!("Destination module not found"))
	}

	fn on_timeout(&self, timeout: Timeout) -> Result<(), anyhow::Error> {
		let (from, source, dest) = match &timeout {
			Timeout::Request(Request::Post(post)) => {
				if post.source != HostStateMachine::get() {
					TokenGatewayInspector::handle_timeout(post)?;
				}
				(&post.from, &post.source, &post.dest)
			},
			Timeout::Request(Request::Get(get)) => (&get.from, &get.source, &get.dest),
			Timeout::Response(res) =>
				(&res.source_module(), &res.source_chain(), &res.dest_chain()),
		};

		if *source != HostStateMachine::get() {
			return Ok(());
		}

		let pallet_id = ModuleId::from_bytes(from).map_err(|err| Error::Custom(err.to_string()))?;
		let xcm_gateway = ModuleId::Evm(XcmGateway::token_gateway_address(dest));
		match pallet_id {
			id if id == xcm_gateway =>
				pallet_xcm_gateway::Module::<Runtime>::default().on_timeout(timeout),
			// instead of returning an error, do nothing. The timeout is for a connected chain.
			_ => Ok(()),
		}
	}
}

#[derive(Default)]
pub struct Router;

impl IsmpRouter for Router {
	fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		Ok(Box::new(ProxyModule::default()))
	}
}
