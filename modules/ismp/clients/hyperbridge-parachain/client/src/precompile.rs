#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use polkadot_sdk::*;
use alloc::vec::Vec;
use core::{marker::PhantomData, num::NonZero};
use frame_support::{traits::Get, weights::Weight};
use pallet_revive::precompiles::{
    alloy::{
        primitives::{FixedBytes},
        self,
        sol_types::{Revert, SolValue},
    },
    AddressMatcher, Error, Ext, Precompile,
};

use crate::{Config as VerifierConfig, Pallet as VerifierPallet};

alloy::sol!("src/IHyperbridgeVerifier.sol");
use IHyperbridgeVerifier::IHyperbridgeVerifierCalls;

/// Trait that provides weights for the verifier precompile operations.
pub trait VerifierWeightSchedule {
    /// Weight for fetching the latest state commitment.
    fn latest_state_commitment() -> Weight;
}

/// [`pallet_revive::precompiles::Precompile`] implementation for the
/// Hyperbridge ismp parachain pallet.
pub struct VerifierPrecompile<Runtime, WeightSchedule>(
    PhantomData<(Runtime, WeightSchedule)>,
);


impl<Runtime, WeightSchedule> Precompile for VerifierPrecompile<Runtime, WeightSchedule>
where
    Runtime: VerifierConfig + pallet_revive::Config,
    WeightSchedule: VerifierWeightSchedule,
{
    type T = Runtime;

    const MATCHER: AddressMatcher = AddressMatcher::Fixed(NonZero::new(3368).unwrap());

    const HAS_CONTRACT_INFO: bool = false;

    type Interface = IHyperbridgeVerifier::IHyperbridgeVerifierCalls;

    fn call(
        _address: &[u8; 20],
        input: &Self::Interface,
        env: &mut impl Ext<T = Self::T>,
    ) -> Result<Vec<u8>, Error> {
        match input {
            IHyperbridgeVerifier::IHyperbridgeVerifierCalls::latestStateCommitment(
                _call,
            ) => {
                env.charge(WeightSchedule::latest_state_commitment())?;

                let state = VerifierPallet::<Runtime>::hyperbridge_state_commitment_height()
                    .ok_or(Error::Revert(Revert {
                        reason: "No verified state commitment found".into(),
                    }))?;

                let commitment = IHyperbridgeVerifier::StateCommitment {
                    timestamp: state.commitment.timestamp,
                    overlayRoot: FixedBytes(state.commitment.overlay_root.unwrap_or_default().0),
                    stateRoot: FixedBytes(state.commitment.state_root.0),
                };

                let sol_state = IHyperbridgeVerifier::StateCommitmentHeight {
                    commitment,
                    height: state.height,
                };

                Ok(sol_state.abi_encode())
            }
        }
    }
}

