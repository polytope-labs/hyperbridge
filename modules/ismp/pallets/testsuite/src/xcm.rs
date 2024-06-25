// Xcm config

use crate::{
	relay_chain,
	runtime::{
		register_offchain_ext, Assets, Balance, Balances, Ismp, MessageQueue, PalletXcm,
		ParachainInfo, ParachainSystem, RuntimeCall, RuntimeEvent, RuntimeOrigin, System, Test,
		Timestamp, XcmpQueue,
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
use pallet_asset_gateway::{xcm_utilities::HyperbridgeAssetTransactor, TokenGatewayParams};
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain_primitives::primitives::{DmpMessageHandler, Sibling};
use sp_core::{H160, H256};
use sp_runtime::{traits::Identity, AccountId32, BuildStorage, Percent};
use staging_xcm::{latest::prelude::*, VersionedXcm};
use staging_xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, ConvertedConcreteId, EnsureXcmOrigin,
	FixedWeightBounds, NativeAsset, NoChecking, ParentIsPreset, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use staging_xcm_executor::{traits::ConvertLocation, XcmExecutor};
use xcm_simulator::{
	decl_test_network, decl_test_parachain, decl_test_relay_chain, ParaId, TestExt,
};
use xcm_simulator_example::ALICE;

pub type SovereignAccountOf = (
	SiblingParachainConvertsVia<Sibling, AccountId32>,
	AccountId32Aliases<RelayNetwork, AccountId32>,
	ParentIsPreset<AccountId32>,
);

// `EnsureOriginWithArg` impl for `CreateOrigin` which allows only XCM origins
// which are locations containing the class location.
pub struct ForeignCreators;
impl EnsureOriginWithArg<RuntimeOrigin, MultiLocation> for ForeignCreators {
	type Success = AccountId32;

	fn try_origin(
		o: RuntimeOrigin,
		a: &MultiLocation,
	) -> sp_std::result::Result<Self::Success, RuntimeOrigin> {
		let origin_location = pallet_xcm::EnsureXcm::<Everything>::try_origin(o.clone())?;
		if !a.starts_with(&origin_location) {
			return Err(o);
		}
		SovereignAccountOf::convert_location(&origin_location).ok_or(o)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(a: &MultiLocation) -> Result<RuntimeOrigin, ()> {
		Ok(pallet_xcm::Origin::Xcm(a.clone()).into())
	}
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
	pub const ReservedDmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub UniversalLocation: Junctions = Parachain(ParachainInfo::parachain_id().into()).into();
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId32>,
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
	pub ForeignPrefix: MultiLocation = (Parent,).into();
}

pub struct CheckingAccount;

impl Get<AccountId32> for CheckingAccount {
	fn get() -> AccountId32 {
		AccountId32::new([0u8; 32])
	}
}

pub type LocalAssetTransactor = HyperbridgeAssetTransactor<
	Test,
	ConvertedConcreteId<MultiLocation, Balance, Identity, Identity>,
	LocationToAccountId,
	NoChecking,
	CheckingAccount,
>;
pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let asset_id = MultiLocation::parent();
	let config: pallet_assets::GenesisConfig<Test> = pallet_assets::GenesisConfig {
		assets: vec![
			// id, owner, is_sufficient, min_balance
			(asset_id.clone(), ALICE, true, 1),
		],
		metadata: vec![
			// id, name, symbol, decimals
			(asset_id, "Token Name".into(), "TOKEN".into(), 10),
		],
		accounts: vec![],
	};

	let para_config: parachain_info::GenesisConfig<Test> =
		parachain_info::GenesisConfig { _config: Default::default(), parachain_id: para_id.into() };

	config.assimilate_storage(&mut t).unwrap();
	para_config.assimilate_storage(&mut t).unwrap();

	let mut ext = sp_io::TestExternalities::new(t);

	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		System::set_block_number(1);
		Timestamp::set_timestamp(1_000_000);
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
			let id = sp_io::hashing::blake2_256(&data[..]);
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
						let _ = XcmExecutor::<XcmConfig>::execute_xcm(Parent, x.clone(), id, limit);
						println!("Executed Xcm message")
					},
				},
			}
		}
		limit
	}
}

decl_test_parachain! {
	pub struct ParaA {
		Runtime = Test,
		XcmpMessageHandler = XcmpQueue,
		DmpMessageHandler = DmpMessageExecutor,
		new_ext = para_ext(100),
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
			(100, ParaA),
		],
	}
}

pub type XcmRouter = ParachainXcmRouter<ParachainInfo>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl staging_xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = ();
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = ();
	type SubscriptionService = ();
	type PalletInstancesInfo = ();
	type FeeManager = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Nothing;
	type Aliasers = Nothing;
}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl parachain_info::Config for Test {}

impl cumulus_pallet_parachain_system::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Test>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = AnyRelayNumber;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type WeightInfo = ();
	type ConsensusHook = RequireParentIncluded;
}

use frame_support::traits::TransformOrigin;
use parachains_common::message_queue::ParaIdToSibling;
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;

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
	type XcmReserveTransferFilter = Everything;
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
}

parameter_types! {
	pub const AssetPalletId: PalletId = PalletId(*b"asset-tx");
	pub const ProtocolAccount: PalletId = PalletId(*b"protocol");
	pub const TransferParams: TokenGatewayParams = TokenGatewayParams::from_parts(Permill::from_parts(1_000)); // 0.1%
}

impl pallet_asset_gateway::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = AssetPalletId;
	type ProtocolAccount = ProtocolAccount;
	type Params = TransferParams;
	type Assets = Assets;
	type IsmpHost = Ismp;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = MultiLocation;
	type AssetIdParameter = MultiLocation;
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
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = IdentityBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct IdentityBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<MultiLocation> for IdentityBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> MultiLocation {
		MultiLocation::new(1, Parachain(id))
	}
}
