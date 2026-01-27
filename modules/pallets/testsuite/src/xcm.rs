// Xcm config
use crate::{
	relay_chain,
	runtime::{
		register_offchain_ext, Assets, Balance, Balances, Ismp, MessageQueue, PalletXcm,
		ParachainInfo, ParachainSystem, RuntimeCall, RuntimeEvent, RuntimeOrigin, System, Test,
		Timestamp, XcmpQueue, ALICE,
	},
};
use codec::Decode;
use cumulus_pallet_parachain_system::{consensus_hook::RequireParentIncluded, AnyRelayNumber};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
	pallet_prelude::Get,
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU32, EnsureOrigin, EnsureOriginWithArg, Everything,
		Nothing,
	},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
	PalletId,
};
use frame_system::EnsureRoot;
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use pallet_xcm::XcmPassthrough;
use pallet_xcm_gateway::{
	xcm_utilities::{ConvertAssetId, HyperbridgeAssetTransactor, ReserveTransferFilter},
	AssetGatewayParams,
};
use polkadot_parachain_primitives::primitives::{DmpMessageHandler, Sibling};
use polkadot_sdk::{cumulus_pallet_parachain_system::DefaultCoreSelector, *};
use sp_core::H256;
use sp_runtime::{traits::Identity, AccountId32, BuildStorage, Permill};
use staging_xcm::{latest::prelude::*, VersionedXcm};
use staging_xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, ConvertedConcreteId, EnsureXcmOrigin,
	FixedWeightBounds, NativeAsset, NoChecking, ParentIsPreset, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use staging_xcm_executor::{traits::ConvertLocation, WeighedMessage, XcmExecutor};
use std::sync::Arc;
use xcm_simulator::{
	decl_test_network, decl_test_parachain, decl_test_relay_chain, mock_message_queue, ParaId,
	TestExt,
};

pub type SovereignAccountOf = (
	SiblingParachainConvertsVia<Sibling, AccountId32>,
	AccountId32Aliases<RelayNetwork, AccountId32>,
	ParentIsPreset<AccountId32>,
);

// `EnsureOriginWithArg` impl for `CreateOrigin` which allows only XCM origins
// which are locations containing the class location.
pub struct ForeignCreators;
impl EnsureOriginWithArg<RuntimeOrigin, Location> for ForeignCreators {
	type Success = AccountId32;

	fn try_origin(
		o: RuntimeOrigin,
		a: &Location,
	) -> sp_std::result::Result<Self::Success, RuntimeOrigin> {
		let origin_location = pallet_xcm::EnsureXcm::<Everything>::try_origin(o.clone())?;
		if !a.starts_with(&origin_location) {
			return Err(o);
		}
		SovereignAccountOf::convert_location(&origin_location).ok_or(o)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(a: &Location) -> Result<RuntimeOrigin, ()> {
		Ok(pallet_xcm::Origin::Xcm(a.clone()).into())
	}
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
	pub const ReservedDmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
}

parameter_types! {
	pub const RelayLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub UniversalLocation: Junctions = Parachain(ParachainInfo::parachain_id().into()).into();
}

parameter_types! {
	pub ExternalConsensus: InteriorLocation = [Parachain(SIBLING_PARA_ID), Parachain(1000)].into();
}

pub type LocationToAccountId = (
	SiblingParachainConvertsVia<Sibling, AccountId32>,
	AccountId32Aliases<RelayNetwork, AccountId32>,
);

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const UnitWeightCost: Weight = Weight::from_parts(1, 1);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub ForeignPrefix: Location = (Parent,).into();
}

pub struct CheckingAccount;

impl Get<AccountId32> for CheckingAccount {
	fn get() -> AccountId32 {
		AccountId32::new([0u8; 32])
	}
}

pub type LocalAssetTransactor = HyperbridgeAssetTransactor<
	Test,
	ConvertedConcreteId<H256, Balance, ConvertAssetId<Test>, Identity>,
	LocationToAccountId,
	NoChecking,
	CheckingAccount,
>;
pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let para_config: staging_parachain_info::GenesisConfig<Test> =
		staging_parachain_info::GenesisConfig {
			_config: Default::default(),
			parachain_id: para_id.into(),
		};

	para_config.assimilate_storage(&mut t).unwrap();

	let asset_location = Location::new(1, Here);
	let asset_id: H256 = sp_io::hashing::keccak_256(&asset_location.encode()).into();

	let config: pallet_assets::GenesisConfig<Test> = pallet_assets::GenesisConfig {
		assets: vec![
			// id, owner, is_sufficient, min_balance
			(asset_id.clone(), ALICE, true, 1),
		],
		accounts: vec![
			(asset_id, ALICE.into(), 10000_000_000_00000 * 10),
			(asset_id, BOB.into(), 0),
		],
		metadata: vec![
			// id, name, symbol, decimals
			(asset_id, "Token Name".into(), "TOKEN".into(), 10),
		],
		next_asset_id: None,
	};

	config.assimilate_storage(&mut t).unwrap();

	let mut ext = sp_io::TestExternalities::new(t);

	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		System::set_block_number(1);
		Timestamp::set_timestamp(1_000_000);
		crate::runtime::MsgQueue::set_para_id(para_id.into());
	});
	ext
}

// This struct is necessary to execute xcm messages from the relaychain to the parachain in this
// unit test environment, the xcm-simulator MockNet only uses `DmpMessageHandler` for executing
// messages from relaychain to parachain, that trait is no longer implemented in `polkadot-sdk` the
// only other alternative would be running full Integration tests for the prebuilt runtimes with
// xcm-emulator.
pub struct DmpMessageExecutor;

impl DmpMessageHandler for DmpMessageExecutor {
	fn handle_dmp_messages(iter: impl Iterator<Item = (u32, Vec<u8>)>, limit: Weight) -> Weight {
		for (_i, (_sent_at, data)) in iter.enumerate() {
			let mut id = sp_io::hashing::blake2_256(&data[..]);
			let maybe_versioned = VersionedXcm::<RuntimeCall>::decode(&mut &data[..]);
			match maybe_versioned {
				Err(_) => {
					println!("Invalid format")
				},
				Ok(versioned) => match Xcm::try_from(versioned) {
					Err(_) => {
						println!("Unsupported version")
					},
					Ok(x) => {
						let _ = XcmExecutor::<XcmConfig>::execute(
							Parent,
							WeighedMessage::new(Default::default(), x.clone()),
							&mut id,
							limit,
						);
						println!("Executed Xcm message")
					},
				},
			}
		}
		limit
	}
}

/// 1000-2000 are considered system parachains, so let's use higher para_id
pub const SIBLING_PARA_ID: u32 = 2222;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = Test,
		XcmpMessageHandler = crate::runtime::MsgQueue,
		DmpMessageHandler = DmpMessageExecutor,
		new_ext = para_ext(SIBLING_PARA_ID),
	}
}

decl_test_parachain! {
	pub struct ParaB {
	   Runtime = crate::asset_hub_runtime::AssetHubsTest,
	   XcmpMessageHandler = crate::asset_hub_runtime::MsgQueue,
	   DmpMessageHandler = crate::asset_hub_xcm::DmpMessageExecutor,
	   new_ext = crate::asset_hub_xcm::para_ext(1000),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay_chain::Runtime,
		RuntimeCall = relay_chain::RuntimeCall,
		RuntimeEvent = relay_chain::RuntimeEvent,
		XcmConfig = relay_chain::XcmConfig,
		MessageQueue = relay_chain::MessageQueue,
		System = relay_chain::System,
		new_ext = relay_chain::relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(SIBLING_PARA_ID, ParaA),
			(1000, ParaB),
		],
	}
}

pub type XcmRouter = ParachainXcmRouter<crate::runtime::MsgQueue>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

parameter_types! {
	pub DotOnAssetHub: Location = Location::new(1, Here);
}

pub struct TestReserve;
impl ContainsPair<Asset, Location> for TestReserve {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		println!("TestReserve::contains asset: {asset:?}, origin:{origin:?}");
		let self_para = Location::new(1, [Parachain(ParachainInfo::parachain_id().into())]);
		if origin == &self_para {
			return false;
		}

		let asset_hub = Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]);
		if origin == &asset_hub {
			let AssetId(asset_id) = &asset.id;
			return DotOnAssetHub::get() == *asset_id;
		}

		false
	}
}

pub struct XcmConfig;
impl staging_xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = TestReserve;
	type IsTeleporter = (
		// Important setting reflecting AssetHub
		parachains_common::xcm_config::ConcreteAssetFromSystem<RelayLocation>,
	);
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = ();
	type ResponseHandler = ();
	type AssetTrap = PalletXcm;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = PalletXcm;
	type SubscriptionService = ();
	type PalletInstancesInfo = ();
	type FeeManager = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Nothing;
	type Aliasers = Nothing;

	type TransactionalProcessor = ();

	type HrmpNewChannelOpenRequestHandler = ();

	type HrmpChannelAcceptedHandler = ();

	type HrmpChannelClosingHandler = ();

	type XcmRecorder = ();
	type XcmEventEmitter = ();
}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl staging_parachain_info::Config for Test {}

impl cumulus_pallet_parachain_system::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = staging_parachain_info::Pallet<Test>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = AnyRelayNumber;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type WeightInfo = ();
	type ConsensusHook = RequireParentIncluded;
	type SelectCore = DefaultCoreSelector<Self>;
	type RelayParentOffset = ();
}

use crate::runtime::BOB;
use frame_support::traits::TransformOrigin;
use pallet_xcm_gateway::xcm_utilities::ASSET_HUB_PARA_ID;
use parachains_common::message_queue::ParaIdToSibling;
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use polkadot_sdk::{
	frame_support::traits::ContainsPair,
	sp_runtime::traits::AccountIdConversion,
	staging_xcm_builder::ExternalConsensusLocationsConverterFor,
	xcm_simulator::Junctions::{X1, X3},
};

impl cumulus_pallet_xcmp_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PalletXcm;
	type ControllerOrigin = EnsureRoot<AccountId32>;
	type ControllerOriginConverter = XcmOriginToCallOrigin;
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = sp_core::ConstU32<1_000>;
	type WeightInfo = ();
	type MaxActiveOutboundChannels = sp_core::ConstU32<128>;
	type MaxPageSize = sp_core::ConstU32<{ 103 * 1024 }>;
}

impl mock_message_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub MessageQueueServiceWeight: Option<Weight> = None;
}

impl pallet_message_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = staging_xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		staging_xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type HeapSize = sp_core::ConstU32<{ 64 * 1024 }>;
	type MaxStale = sp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueServiceWeight;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId32, RelayNetwork>;

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = ReserveTransferFilter;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId32>;
	type AuthorizedAliasConsideration = ();
}

parameter_types! {
	pub const AssetPalletId: PalletId = PalletId(*b"asset-tx");
	pub const ProtocolAccount: PalletId = PalletId(*b"protocol");
	pub const TransferParams: AssetGatewayParams = AssetGatewayParams::from_parts(Permill::from_parts(1_000)); // 0.1%
}

impl pallet_xcm_gateway::Config for Test {
	type PalletId = AssetPalletId;
	type Params = TransferParams;
	type Assets = Assets;
	type IsmpHost = Ismp;
	type GatewayOrigin = EnsureRoot<AccountId32>;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = H256;
	type AssetIdParameter = H256;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId32>>;
	type ForceOrigin = EnsureRoot<AccountId32>;
	type AssetDeposit = ConstU128<1>;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = ConstU128<1>;
	type MetadataDepositPerByte = ConstU128<1>;
	type ApprovalDeposit = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type WeightInfo = ();
	type CallbackHandle = ();
	type Extra = ();
	type RemoveItemsLimit = ConstU32<5>;
	type Holder = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = IdentityBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct IdentityBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<H256> for IdentityBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> H256 {
		use codec::Encode;
		sp_io::hashing::keccak_256(&Location::new(1, Parachain(id)).encode()).into()
	}
}
