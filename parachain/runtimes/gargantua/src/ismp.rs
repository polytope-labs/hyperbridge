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
	weights, AccountId, Assets, Balance, Balances, Fishermen, Ismp, IsmpParachain, Mmr,
	ParachainInfo, Runtime, RuntimeEvent, Timestamp, TreasuryPalletId, EXISTENTIAL_DEPOSIT,
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
	router::{GetResponse, IsmpRouter, PostRequest, Request},
};
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use polkadot_sdk::{sp_weights::WeightToFee, *};
use sp_core::{crypto::AccountId32, H256};

use ismp::consensus::StateMachineClient;
use ismp_sync_committee::constants::{gnosis, sepolia::Sepolia};
use pallet_ismp::{dispatcher::FeeMetadata, ModuleId};
use polkadot_sdk::sp_runtime::Weight;
use sp_std::prelude::*;
use substrate_state_machine::SubstrateStateMachine;

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

/// Bandwidth gate the runtime exposes to consumers (currently
/// `pallet-state-coprocessor`). With the `no-bandwidth` flag off (the
/// default) the live `pallet-bandwidth` pallet is consulted; turn the
/// flag on and a no-op gate is wired in so the trait bound on
/// `pallet_state_coprocessor::Config` is satisfied without enforcing
/// per-app quotas.
#[cfg(not(feature = "no-bandwidth"))]
type RuntimeBandwidthGate = pallet_bandwidth::Pallet<Runtime>;
#[cfg(feature = "no-bandwidth")]
type RuntimeBandwidthGate = NoopBandwidthGate;

#[cfg(feature = "no-bandwidth")]
pub struct NoopBandwidthGate;
#[cfg(feature = "no-bandwidth")]
impl pallet_bandwidth::BandwidthGate for NoopBandwidthGate {
	fn try_consume(
		_source: &StateMachine,
		_app: &[u8],
		_bytes: u32,
	) -> Result<(), pallet_bandwidth::GateError> {
		Ok(())
	}
}

impl pallet_state_coprocessor::Config for Runtime {
	type IsmpHost = Ismp;
	type Mmr = Mmr;
	type BandwidthGate = RuntimeBandwidthGate;
}

parameter_types! {
	pub const IntentStorageDepositFee: Balance = 100 * EXISTENTIAL_DEPOSIT;
	pub const IntentPhantomOrderBidWindow: u32 = 5;
}

impl pallet_intents_coprocessor::Config for Runtime {
	type Dispatcher = Ismp;
	type Currency = Balances;
	type StorageDepositFee = IntentStorageDepositFee;
	type PhantomOrderBidWindowBlocks = IntentPhantomOrderBidWindow;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_intents_coprocessor::WeightInfo<Runtime>;
}

impl ismp_arbitrum::pallet::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
	type FishermanBlacklist = Fishermen;
}

impl ismp_optimism::pallet::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type IsmpHost = Ismp;
	type FishermanBlacklist = Fishermen;
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
			_ => Ok(Box::new(SubstrateStateMachine::<Runtime>::from(id))),
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
		ismp_grandpa::consensus::GrandpaConsensusClient<Runtime>,
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

impl pallet_ismp_demo::Config for Runtime {
	type Balance = Balance;
	type NativeCurrency = Balances;
	type IsmpHost = Ismp;
}

impl pallet_ismp_relayer::Config for Runtime {
	type IsmpHost = Ismp;
	type RelayerOrigin = EnsureRoot<AccountId>;
	type TreasuryPalletId = TreasuryPalletId;
}

impl pallet_ismp_host_executive::Config for Runtime {
	type IsmpHost = Ismp;
	type HostExecutiveOrigin = EnsureRoot<AccountId>;
}

impl pallet_call_decompressor::Config for Runtime {
	type MaxCallSize = ConstU32<2>;
	type WeightInfo = pallet_call_decompressor::weights::SubstrateWeight<Runtime>;
}

impl ismp_parachain::Config for Runtime {
	type IsmpHost = Ismp;
	type WeightInfo = weights::ismp_parachain::WeightInfo<Runtime>;
	type RootOrigin = EnsureRoot<AccountId>;
}

impl ismp_beefy::BeefyClientConfig for Runtime {
	fn is_parachain_tracked(para_id: u32) -> bool {
		para_id == 4009
	}

	fn sp1_vkey_hash() -> sp_core::H256 {
		pallet_beefy_consensus_proofs::Sp1VkeyHash::<Runtime>::get()
	}

	fn allowed_proof_types() -> &'static [u8] {
		// Testnet: accept both the naive ECDSA and SP1 ZK proof formats.
		&[ismp_beefy::PROOF_TYPE_NAIVE, ismp_beefy::PROOF_TYPE_SP1]
	}
}

/// True when the account is registered with `pallet-collator-selection` as
/// an invulnerable or as a bonded candidate. Active session membership is
/// not required: a freshly registered candidate who hasn't been selected for
/// the current session is still a legitimate fisherman. Candidates that have
/// called `leave_intent` are removed from `CandidateList` in the same block,
/// so being in this list also implies "has not declared intent to withdraw."
pub struct IsCollator;
impl frame_support::traits::Contains<AccountId> for IsCollator {
	fn contains(account: &AccountId) -> bool {
		if pallet_collator_selection::Invulnerables::<Runtime>::get().contains(account) {
			return true;
		}
		false
	}
}

impl pallet_fishermen::Config for Runtime {
	type IsmpHost = Ismp;
	type IsCollator = IsCollator;
}

#[cfg(not(feature = "no-bandwidth"))]
impl pallet_bandwidth::Config for Runtime {
	type Dispatcher = Ismp;
}

parameter_types! {
	pub const HftDecimals: u8 = 10;
}

pub struct HftNativeAssetId;

impl Get<H256> for HftNativeAssetId {
	fn get() -> H256 {
		sp_io::hashing::keccak_256(b"BRIDGE").into()
	}
}

impl pallet_hyper_fungible_token::Config for Runtime {
	type Dispatcher = Ismp;
	type Assets = Assets;
	type NativeCurrency = Balances;
	type NativeAssetId = HftNativeAssetId;
	type CreateOrigin = EnsureRoot<AccountId>;
	type Decimals = HftDecimals;
	type EvmToSubstrate = ();
	type WeightInfo = ();
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
		// Bandwidth gate. Always-enforce unless the `no-bandwidth` flag
		// is set; skipped for purchase messages so the recharge flow
		// itself doesn't need bandwidth. With the flag on the gate is a
		// no-op and this block is compiled out entirely.
		#[cfg(not(feature = "no-bandwidth"))]
		if !pallet_bandwidth::Pallet::<Runtime>::is_purchase_message(&request) {
			let bytes = ismp::abi::encode_post_request(&request).len() as u32;
			<pallet_bandwidth::Pallet<Runtime> as pallet_bandwidth::BandwidthGate>::try_consume(
				&request.source,
				&request.from,
				bytes,
			)
			.map_err(|err| {
				anyhow!(
					"bandwidth gate: {err} (source={:?}, from={:x?})",
					request.source,
					request.from
				)
			})?;
		}

		if request.dest != HostStateMachine::get() {
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

			#[cfg(not(feature = "no-bandwidth"))]
			id if id == ModuleId::Pallet(pallet_bandwidth::pallet::PALLET_BANDWIDTH) =>
				pallet_bandwidth::Pallet::<Runtime>::default().on_accept(request),

			pallet_hyper_fungible_token::PALLET_ID =>
				pallet_hyper_fungible_token::Pallet::<Runtime>::default().on_accept(request),

			_ => Err(anyhow!("Destination module not found")),
		}
	}

	fn on_response(&self, response: GetResponse) -> Result<Weight, anyhow::Error> {
		// Bandwidth gate. Mirrors the request path in `on_accept`: the chain
		// and module that produced the response pay for the bytes they
		// deliver. Compiled out when the `no-bandwidth` flag is on.
		if response.dest_chain() != HostStateMachine::get() {
			return Ok(Weight::from_parts(0, 0));
		}

		let dest = &response.get.from;

		let pallet_id = ModuleId::from_bytes(dest).map_err(|err| Error::Custom(err.to_string()))?;

		match pallet_id {
			pallet_ismp_demo::PALLET_ID =>
				pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_response(response),
			_ => Err(anyhow!("Destination module not found")),
		}
	}

	fn on_timeout(&self, timeout: Request) -> Result<Weight, anyhow::Error> {
		let (from, source) = match &timeout {
			Request::Post(post) => (&post.from, post.source.clone()),
			Request::Get(get) => (&get.from, get.source.clone()),
		};

		if source != HostStateMachine::get() {
			return Ok(Weight::from_parts(0, 0));
		}

		let pallet_id = ModuleId::from_bytes(from).map_err(|err| Error::Custom(err.to_string()))?;
		match pallet_id {
			pallet_ismp_demo::PALLET_ID =>
				pallet_ismp_demo::IsmpModuleCallback::<Runtime>::default().on_timeout(timeout),
			pallet_hyper_fungible_token::PALLET_ID =>
				pallet_hyper_fungible_token::Pallet::<Runtime>::default().on_timeout(timeout),
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
