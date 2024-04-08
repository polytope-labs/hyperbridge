// Xcm config

use crate::{
    relay_chain,
    runtime::{
        register_offchain_ext, Assets, Balance, Balances, MsgQueue, RuntimeCall, RuntimeEvent,
        RuntimeOrigin, System, Test, Timestamp,
    },
};
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
use pallet_asset_transfer::xcm_utilities::HyperbridgeAssetTransactor;
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain_primitives::primitives::{
    DmpMessageHandler, Sibling, XcmpMessageFormat, XcmpMessageHandler,
};
use sp_core::{H160, H256};
use sp_runtime::{
    traits::{Hash, Identity},
    AccountId32, BuildStorage, Percent,
};
use staging_xcm::latest::prelude::*;
use staging_xcm_builder::{
    AccountId32Aliases, AllowUnpaidExecutionFrom, ConvertedConcreteId, EnsureXcmOrigin,
    FixedWeightBounds, NativeAsset, NoChecking, ParentIsPreset, SiblingParachainConvertsVia,
    SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use staging_xcm_executor::{traits::ConvertLocation, XcmExecutor};
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};
use xcm_simulator_example::ALICE;

#[frame_support::pallet]
pub mod mock_msg_queue {
    use super::*;
    use frame_support::pallet_prelude::*;
    use staging_xcm::VersionedXcm;
    use xcm_simulator::{ParaId, RelayBlockNumber};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn parachain_id)]
    pub(super) type ParachainId<T: Config> = StorageValue<_, ParaId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn received_dmp)]
    /// A queue of received DMP messages
    pub(super) type ReceivedDmp<T: Config> = StorageValue<_, Vec<Xcm<T::RuntimeCall>>, ValueQuery>;

    impl<T: Config> Get<ParaId> for Pallet<T> {
        fn get() -> ParaId {
            Self::parachain_id()
        }
    }

    pub type MessageId = [u8; 32];

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // XCMP
        /// Some XCM was executed OK.
        Success(Option<T::Hash>),
        /// Some XCM failed.
        Fail(Option<T::Hash>, XcmError),
        /// Bad XCM version used.
        BadVersion(Option<T::Hash>),
        /// Bad XCM format used.
        BadFormat(Option<T::Hash>),

        // DMP
        /// Downward message is invalid XCM.
        InvalidFormat(MessageId),
        /// Downward message is unsupported version of XCM.
        UnsupportedVersion(MessageId),
        /// Downward message executed with the given outcome.
        ExecutedDownward(MessageId, Outcome),
    }

    impl<T: Config> Pallet<T> {
        pub fn set_para_id(para_id: ParaId) {
            ParachainId::<T>::put(para_id);
        }

        fn handle_xcmp_message(
            sender: ParaId,
            _sent_at: RelayBlockNumber,
            xcm: VersionedXcm<T::RuntimeCall>,
            max_weight: Weight,
        ) -> Result<Weight, XcmError> {
            let hash = Encode::using_encoded(&xcm, T::Hashing::hash);
            let mut message_hash = Encode::using_encoded(&xcm, sp_io::hashing::blake2_256);
            let (result, event) = match Xcm::<T::RuntimeCall>::try_from(xcm) {
                Ok(xcm) => {
                    let location = (Parent, Parachain(sender.into()));
                    match T::XcmExecutor::prepare_and_execute(
                        location,
                        xcm,
                        &mut message_hash,
                        max_weight,
                        Weight::zero(),
                    ) {
                        Outcome::Error(error) => (Err(error), Event::Fail(Some(hash), error)),
                        Outcome::Complete(used) => (Ok(used), Event::Success(Some(hash))),
                        // As far as the caller is concerned, this was dispatched without error, so
                        // we just report the weight used.
                        Outcome::Incomplete(used, error) =>
                            (Ok(used), Event::Fail(Some(hash), error)),
                    }
                },
                Err(()) => (Err(XcmError::UnhandledXcmVersion), Event::BadVersion(Some(hash))),
            };
            Self::deposit_event(event);
            result
        }
    }

    impl<T: Config> XcmpMessageHandler for Pallet<T> {
        fn handle_xcmp_messages<'a, I: Iterator<Item = (ParaId, RelayBlockNumber, &'a [u8])>>(
            iter: I,
            max_weight: Weight,
        ) -> Weight {
            for (sender, sent_at, data) in iter {
                let mut data_ref = data;
                let _ = XcmpMessageFormat::decode(&mut data_ref)
                    .expect("Simulator encodes with versioned xcm format; qed");

                let mut remaining_fragments = data_ref;
                while !remaining_fragments.is_empty() {
                    if let Ok(xcm) =
                        VersionedXcm::<T::RuntimeCall>::decode(&mut remaining_fragments)
                    {
                        let _ = Self::handle_xcmp_message(sender, sent_at, xcm, max_weight);
                    } else {
                        debug_assert!(false, "Invalid incoming XCMP message data");
                    }
                }
            }
            max_weight
        }
    }

    impl<T: Config> DmpMessageHandler for Pallet<T> {
        fn handle_dmp_messages(
            iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
            limit: Weight,
        ) -> Weight {
            for (_i, (_sent_at, data)) in iter.enumerate() {
                let mut id = sp_io::hashing::blake2_256(&data[..]);
                let maybe_versioned = VersionedXcm::<T::RuntimeCall>::decode(&mut &data[..]);
                match maybe_versioned {
                    Err(_) => {
                        Self::deposit_event(Event::InvalidFormat(id));
                    },
                    Ok(versioned) => match Xcm::try_from(versioned) {
                        Err(()) => Self::deposit_event(Event::UnsupportedVersion(id)),
                        Ok(x) => {
                            let outcome = T::XcmExecutor::prepare_and_execute(
                                Parent,
                                x.clone(),
                                &mut id,
                                limit,
                                Weight::zero(),
                            );
                            <ReceivedDmp<T>>::append(x);
                            Self::deposit_event(Event::ExecutedDownward(id, outcome));
                        },
                    },
                }
            }
            limit
        }
    }
}

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
            return Err(o)
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
    pub UniversalLocation: Junctions = Parachain(MsgQueue::parachain_id().into()).into();
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
    config.assimilate_storage(&mut t).unwrap();

    let mut ext = sp_io::TestExternalities::new(t);

    register_offchain_ext(&mut ext);
    ext.execute_with(|| {
        System::set_block_number(1);
        Timestamp::set_timestamp(1_000_000);
        MsgQueue::set_para_id(para_id.into());
    });
    ext
}

decl_test_parachain! {
    pub struct ParaA {
        Runtime = Test,
        XcmpMessageHandler = MsgQueue,
        DmpMessageHandler = MsgQueue,
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

pub type XcmRouter = ParachainXcmRouter<MsgQueue>;
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

impl mock_msg_queue::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
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
    pub const TokenGateWay: H160 = H160::zero();
    pub const DotAssetId: H256 = H256::zero();
    pub const ProtocolFees: Percent = Percent::from_percent(1);
}

impl pallet_asset_transfer::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = AssetPalletId;
    type ProtocolAccount = ProtocolAccount;
    type TokenGateWay = TokenGateWay;
    type DotAssetId = DotAssetId;
    type ProtocolFees = ProtocolFees;
    type EvmAccountId = H160;
    type Assets = Assets;
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
