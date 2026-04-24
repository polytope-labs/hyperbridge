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
	weights, AccountId, Balance, Balances, Ismp, IsmpParachain, Mmr, ParachainInfo, Runtime,
	RuntimeEvent, Timestamp, TokenGatewayInspector, TreasuryPalletId, EXISTENTIAL_DEPOSIT,
};
use anyhow::anyhow;
use evm_state_machine::SubstrateEvmStateMachine;
use frame_support::{
	pallet_prelude::{ConstU32, Get},
	parameter_types,
	traits::AsEnsureOriginWithArg,
};
use frame_system::EnsureRoot;
use ismp::{
	error::Error,
	host::StateMachine,
	module::IsmpModule,
	router::{IsmpRouter, PostRequest, Request, Response},
};
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use polkadot_sdk::{sp_weights::WeightToFee, *};
use sp_core::{crypto::AccountId32, H256};

use hyperbridge_client_machine::HyperbridgeClientMachine;
use ismp::{consensus::StateMachineClient, router::Timeout};
use ismp_sync_committee::constants::{gnosis, sepolia::Sepolia};
use pallet_ismp::{dispatcher::FeeMetadata, ModuleId};
use polkadot_sdk::sp_runtime::Weight;
use sp_std::prelude::*;

#[derive(Default)]
pub struct ProxyModule;

pub struct HostStateMachine;

impl Get<StateMachine> for HostStateMachine {
	fn get() -> StateMachine {
		StateMachine::Kusama(ParachainInfo::get().into())
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

impl ismp_bsc::pallet::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
}

impl pallet_state_coprocessor::Config for Runtime {
	type IsmpHost = Ismp;
	type Mmr = Mmr;
}

parameter_types! {
	pub const IntentStorageDepositFee: Balance = 100 * EXISTENTIAL_DEPOSIT;
}

impl pallet_intents_coprocessor::Config for Runtime {
	type Dispatcher = Ismp;
	type Currency = Balances;
	type StorageDepositFee = IntentStorageDepositFee;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_intents_coprocessor::WeightInfo<Runtime>;
}

impl ismp_arbitrum::pallet::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
}

impl ismp_optimism::pallet::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
}

impl ismp_tendermint::pallet::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
}

pub struct Coprocessor;

impl Get<Option<StateMachine>> for Coprocessor {
	fn get() -> Option<StateMachine> {
		Some(HostStateMachine::get())
	}
}

pub struct ParachainStateMachineProvider;

impl ismp_parachain::ParachainStateMachineProvider<Runtime> for ParachainStateMachineProvider {
	fn state_machine(id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match id {
			StateMachine::Evm(chain_id)
				if chain_id == ismp_parachain::PASSET_HUB_TESTNET_CHAIN_ID =>
				Ok(Box::new(SubstrateEvmStateMachine::<Ismp, Runtime>::default())),
			_ => Ok(Box::new(HyperbridgeClientMachine::<Runtime, Ismp, ()>::from(id))),
		}
	}
}

pub struct IsmpWeightToFee;
impl WeightToFee for IsmpWeightToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		<Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(&weight)
	}
}

impl pallet_ismp::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type HostStateMachine = HostStateMachine;
	type Coprocessor = Coprocessor;
	type TimestampProvider = Timestamp;
	type Balance = Balance;
	type Currency = Balances;
	type Router = Router;
	type ConsensusClients = (
		ismp_bsc::BscClient<Ismp, Runtime, ismp_bsc::Testnet>,
		ismp_sync_committee::SyncCommitteeConsensusClient<Ismp, Sepolia, Runtime, Ethereum>,
		ismp_sync_committee::SyncCommitteeConsensusClient<Ismp, gnosis::Testnet, Runtime, Gnosis>,
		ismp_parachain::ParachainConsensusClient<
			Runtime,
			IsmpParachain,
			ParachainStateMachineProvider,
		>,
		ismp_grandpa::consensus::GrandpaConsensusClient<
			Runtime,
			HyperbridgeClientMachine<Runtime, Ismp, ()>,
		>,
		ismp_arbitrum::ArbitrumConsensusClient<Ismp, Runtime>,
		ismp_optimism::OptimismConsensusClient<Ismp, Runtime>,
		ismp_polygon::PolygonClient<Ismp, Runtime>,
		ismp_tendermint::TendermintClient<Ismp, Runtime>,
		ismp_pharos::PharosClient<Ismp, Runtime, ismp_pharos::Testnet>,
		ismp_beefy::BeefyConsensusClient<Ismp, Runtime>,
	);
	type OffchainDB = Mmr;
	type FeeHandler = pallet_ismp::fee_handler::WeightFeeHandler<
		AccountId,
		Balances,
		IsmpWeightToFee,
		TreasuryPalletId,
		false,
	>;
	type MigrationWeightInfo = crate::weights::pallet_ismp::WeightInfo<Runtime>;
}

impl ismp_grandpa::Config for Runtime {
	type IsmpHost = pallet_ismp::Pallet<Runtime>;
	type WeightInfo = weights::ismp_grandpa::WeightInfo<Runtime>;
	type RootOrigin = EnsureRoot<AccountId>;
}

impl pallet_token_governor::Config for Runtime {
	type Dispatcher = Ismp;
	type TreasuryAccount = TreasuryPalletId;
	type GovernorOrigin = EnsureRoot<AccountId>;
}

impl pallet_ismp_demo::Config for Runtime {
	type Balance = Balance;
	type NativeCurrency = Balances;
	type IsmpHost = Ismp;
}

/// Wires `pallet-ismp-relayer`'s `RotationOracle` to the on-chain
/// `pallet-beefy-consensus-proofs::RotationProofs` map. The lookup is what
/// gates the outbound consensus delivery reward: a claim's `(set_id,
/// rotation_height)` must match an entry here for the pallet to pay out.
pub struct BeefyRotationOracle;
impl pallet_ismp_relayer::RotationOracle for BeefyRotationOracle {
	fn rotation_height(set_id: u64) -> Option<u64> {
		pallet_beefy_consensus_proofs::RotationProofs::<Runtime>::get()
			.get(&set_id)
			.copied()
	}
}

impl pallet_ismp_relayer::Config for Runtime {
	type IsmpHost = Ismp;
	type RelayerOrigin = EnsureRoot<AccountId>;
	type TreasuryPalletId = TreasuryPalletId;
	type RotationOracle = BeefyRotationOracle;
}

impl pallet_ismp_host_executive::Config for Runtime {
	type IsmpHost = Ismp;
	type HostExecutiveOrigin = EnsureRoot<AccountId>;
}

impl pallet_call_decompressor::Config for Runtime {
	type MaxCallSize = ConstU32<2>;
}

impl ismp_parachain::Config for Runtime {
	type IsmpHost = Ismp;
	type WeightInfo = weights::ismp_parachain::WeightInfo<Runtime>;
	type RootOrigin = EnsureRoot<AccountId>;
}

impl ismp_beefy::BeefyClientConfig for Runtime {
	fn is_parachain_tracked(para_id: u32) -> bool {
		para_id == 4009 || para_id == 3367
	}

	fn sp1_vkey_hash() -> Vec<u8> {
		pallet_beefy_consensus_proofs::Sp1VkeyHash::<Runtime>::get()
	}
}

impl pallet_fishermen::Config for Runtime {
	type IsmpHost = Ismp;
	type FishermenOrigin = EnsureRoot<AccountId>;
}

impl pallet_token_gateway_inspector::Config for Runtime {
	type GatewayOrigin = EnsureRoot<AccountId>;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<H256, ()> for XcmBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> H256 {
		use codec::Encode;
		use staging_xcm::{prelude::Location, v5::Junction::Parachain};
		sp_io::hashing::keccak_256(&Location::new(1, Parachain(id)).encode()).into()
	}

	fn create_reserve_id_parameter(_id: u32) -> () {
		()
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
	type Holder = ();
	type ReserveData = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = XcmBenchmarkHelper;
}

impl IsmpModule for ProxyModule {
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		if request.dest != HostStateMachine::get() {
			TokenGatewayInspector::inspect_request(&request)?;

			Ismp::dispatch_request(
				Request::Post(request),
				FeeMetadata::<Runtime> { payer: [0u8; 32].into(), fee: Default::default() },
			)?;
			return Ok(Weight::from_parts(0, 0));
		}

		let pallet_id =
			ModuleId::from_bytes(&request.to).map_err(|err| Error::Custom(err.to_string()))?;

		match pallet_id {
			pallet_ismp_demo::PALLET_ID =>
				pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_accept(request),
			_ => Err(anyhow!("Destination module not found")),
		}
	}

	fn on_response(&self, response: Response) -> Result<Weight, anyhow::Error> {
		if response.dest_chain() != HostStateMachine::get() {
			Ismp::dispatch_response(
				response,
				FeeMetadata::<Runtime> { payer: [0u8; 32].into(), fee: Default::default() },
			)?;
			return Ok(Weight::from_parts(0, 0));
		}

		let dest = match &response {
			Response::Post(post) => &post.destination_module(),
			Response::Get(resp) => &resp.get.from,
		};

		let pallet_id = ModuleId::from_bytes(dest).map_err(|err| Error::Custom(err.to_string()))?;

		match pallet_id {
			pallet_ismp_demo::PALLET_ID =>
				pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_response(response),
			_ => Err(anyhow!("Destination module not found")),
		}
	}

	fn on_timeout(&self, timeout: Timeout) -> Result<Weight, anyhow::Error> {
		let (from, source) = match &timeout {
			Timeout::Request(Request::Post(post)) => {
				if post.source != HostStateMachine::get() {
					TokenGatewayInspector::handle_timeout(post)?;
				}
				(&post.from, post.source.clone())
			},
			Timeout::Request(Request::Get(get)) => (&get.from, get.source.clone()),
			Timeout::Response(res) => (&res.source_module(), res.source_chain()),
		};

		if source != HostStateMachine::get() {
			return Ok(Weight::from_parts(0, 0));
		}

		let pallet_id = ModuleId::from_bytes(from).map_err(|err| Error::Custom(err.to_string()))?;
		match pallet_id {
			pallet_ismp_demo::PALLET_ID =>
				pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_timeout(timeout),
			// instead of returning an error, do nothing. The timeout is for a connected chain.
			_ => Ok(Weight::from_parts(0, 0)),
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
