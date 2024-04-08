use crate::{Config, Pallet};
use core::marker::PhantomData;
use frame_support::traits::fungibles::{self, Mutate};
use ismp::host::{Ethereum, StateMachine};
use sp_core::Get;
use staging_xcm::{
    prelude::MultiLocation,
    v3::{
        Error as XcmError, Junction, Junctions, MultiAsset, NetworkId, Result as XcmResult,
        XcmContext,
    },
};
use staging_xcm_builder::{AssetChecking, FungiblesMutateAdapter};
use staging_xcm_executor::{
    traits::{ConvertLocation, Error as MatchError, MatchesFungibles, TransactAsset},
    Assets as XcmAssets,
};

// Supported EVM chains
const ARBITRUM_CHAIN_ID: u64 = 42161;
const OPTIMISM_CHAIN_ID: u64 = 10;
const BASE_CHAIN_ID: u64 = 8453;
const ETHEREUM_CHAIN_ID: u64 = 1;
const BSC_CHAIN_ID: u64 = 56;

const ARBITRUM_SEPOLIA_CHAIN_ID: u64 = 421614;
const OPTIMISM_SEPOLIA_CHAIN_ID: u64 = 11155420;
const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;
const SEPOLIA_CHAIN_ID: u64 = 11155111;
const BSC_TESTNET_CHAIN_ID: u64 = 97;

pub struct WrappedNetworkId(pub NetworkId);

impl WrappedNetworkId {
    pub fn transform_to_state_machine(self) -> Option<StateMachine> {
        match self.0 {
            NetworkId::Ethereum { chain_id } => match chain_id {
                ARBITRUM_CHAIN_ID | ARBITRUM_SEPOLIA_CHAIN_ID =>
                    Some(StateMachine::Ethereum(Ethereum::Arbitrum)),
                OPTIMISM_CHAIN_ID | OPTIMISM_SEPOLIA_CHAIN_ID =>
                    Some(StateMachine::Ethereum(Ethereum::Optimism)),
                BASE_CHAIN_ID | BASE_SEPOLIA_CHAIN_ID =>
                    Some(StateMachine::Ethereum(Ethereum::Base)),
                ETHEREUM_CHAIN_ID | SEPOLIA_CHAIN_ID =>
                    Some(StateMachine::Ethereum(Ethereum::ExecutionLayer)),
                BSC_CHAIN_ID | BSC_TESTNET_CHAIN_ID => Some(StateMachine::Bsc),
                _ => None,
            },
            // Only transforms ethereum network ids
            _ => None,
        }
    }
}

/// Converts a MutiLocation to a substrate account and an evm account if the multilocation
/// description matches a supported Ismp State machine
pub struct MultilocationToMultiAccount<A, B>(PhantomData<(A, B)>);

pub struct MultiAccount<A, B> {
    /// Origin substrate account
    pub substrate_account: A,
    /// Destination evm account
    pub evm_account: B,
    /// Destination state machine
    pub dest_state_machine: StateMachine,
}

impl<A: From<[u8; 32]> + Into<[u8; 32]> + Clone, B: From<[u8; 20]> + Into<[u8; 20]> + Clone>
    ConvertLocation<MultiAccount<A, B>> for MultilocationToMultiAccount<A, B>
{
    fn convert_location(location: &MultiLocation) -> Option<MultiAccount<A, B>> {
        // We only support locations X2 Junctions addressed to our parachain and an ethereum account
        match location {
            MultiLocation {
                parents: 0,
                interior:
                    Junctions::X2(
                        Junction::AccountId32 { network: None, id },
                        Junction::AccountKey20 { network: Some(network), key },
                    ),
            } => {
                // Ensure that the network Id is one of the supported ethereum networks
                // If it transforms correctly we return the ethereum account
                let dest_state_machine =
                    WrappedNetworkId(network.clone()).transform_to_state_machine()?;
                Some(MultiAccount {
                    substrate_account: A::from(*id),
                    evm_account: B::from(*key),
                    dest_state_machine,
                })
            },
            // Any other multilocation format is unsupported
            _ => None,
        }
    }
}

pub struct HyperbridgeAssetTransactor<T, Matcher, AccountIdConverter, CheckAsset, CheckingAccount>(
    PhantomData<(T, Matcher, AccountIdConverter, CheckAsset, CheckingAccount)>,
);

impl<
        T: Config,
        Matcher: MatchesFungibles<
            <T::Assets as fungibles::Inspect<T::AccountId>>::AssetId,
            <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
        >,
        AccountIdConverter: ConvertLocation<T::AccountId>,
        CheckAsset: AssetChecking<<T::Assets as fungibles::Inspect<T::AccountId>>::AssetId>,
        CheckingAccount: Get<T::AccountId>,
    > TransactAsset
    for HyperbridgeAssetTransactor<T, Matcher, AccountIdConverter, CheckAsset, CheckingAccount>
where
    <T::Assets as fungibles::Inspect<T::AccountId>>::Balance: Into<u128> + From<u128>,
    u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
    T::AccountId: Eq + Clone + From<[u8; 32]> + Into<[u8; 32]>,
    T::EvmAccountId: Eq + Clone + From<[u8; 20]> + Into<[u8; 20]>,
{
    fn can_check_in(origin: &MultiLocation, what: &MultiAsset, context: &XcmContext) -> XcmResult {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::can_check_in(origin, what, context)
    }

    fn check_in(origin: &MultiLocation, what: &MultiAsset, context: &XcmContext) {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::check_in(origin, what, context)
    }

    fn can_check_out(dest: &MultiLocation, what: &MultiAsset, context: &XcmContext) -> XcmResult {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::can_check_out(dest, what, context)
    }

    fn check_out(dest: &MultiLocation, what: &MultiAsset, context: &XcmContext) {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::check_out(dest, what, context)
    }

    fn deposit_asset(
        what: &MultiAsset,
        who: &MultiLocation,
        _context: Option<&XcmContext>,
    ) -> XcmResult {
        // Check we handle this asset.
        let (asset_id, amount) = Matcher::matches_fungibles(what)?;
        // Regular XCM transaction
        if let Some(who) = AccountIdConverter::convert_location(who) {
            T::Assets::mint_into(asset_id, &who, amount)
                .map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
        }
        // Ismp xcm transaction
        else if let Some(who) =
            MultilocationToMultiAccount::<T::AccountId, T::EvmAccountId>::convert_location(who)
        {
            // We would remove the protocol fee at this point

            let protocol_account = Pallet::<T>::protocol_account_id();
            let pallet_account = Pallet::<T>::account_id();

            let protocol_fees = <T as Config>::ProtocolFees::get() * u128::from(amount);
            let remainder = amount - protocol_fees.into();
            // We dispatch an ismp request to the destination chain
            Pallet::<T>::dispatch_request(who, remainder)
                .map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
            // Mint protocol fees
            T::Assets::mint_into(asset_id.clone(), &protocol_account, protocol_fees.into())
                .map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
            // We custody the funds in the pallet account
            T::Assets::mint_into(asset_id, &pallet_account, remainder)
                .map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
        } else {
            Err(MatchError::AccountIdConversionFailed)?
        }

        Ok(())
    }

    fn withdraw_asset(
        what: &MultiAsset,
        who: &MultiLocation,
        maybe_context: Option<&XcmContext>,
    ) -> Result<XcmAssets, XcmError> {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::withdraw_asset(what, who, maybe_context)
    }

    fn internal_transfer_asset(
        asset: &MultiAsset,
        from: &MultiLocation,
        to: &MultiLocation,
        context: &XcmContext,
    ) -> Result<XcmAssets, XcmError> {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::internal_transfer_asset(asset, from, to, context)
    }

    fn transfer_asset(
        asset: &MultiAsset,
        from: &MultiLocation,
        to: &MultiLocation,
        context: &XcmContext,
    ) -> Result<XcmAssets, XcmError> {
        FungiblesMutateAdapter::<
            T::Assets,
            Matcher,
            AccountIdConverter,
            T::AccountId,
            CheckAsset,
            CheckingAccount,
        >::transfer_asset(asset, from, to, context)
    }
}
