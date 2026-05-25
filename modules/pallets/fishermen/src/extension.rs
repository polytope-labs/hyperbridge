// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Transaction extension that gives fishermen calls (`veto_state_commitment`,
//! `blacklist_dispute_game`, `blacklist_arbitrum_claim`) the highest pool
//! priority, so that the basic-authorship proposer drains them before any
//! normally-priced extrinsic. Every other call passes through with the
//! default priority.

use crate::pallet::{Call, Config};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::traits::{Contains, IsSubType};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use sp_runtime::{
	impl_tx_ext_default,
	traits::{DispatchInfoOf, TransactionExtension, ValidateResult},
	transaction_validity::{TransactionPriority, TransactionSource, ValidTransaction},
	Weight,
};

/// Bumps `pallet_fishermen` fisherman calls (`veto_state_commitment`,
/// `blacklist_dispute_game`, `blacklist_arbitrum_claim`) to
/// [`TransactionPriority::MAX`] in the transaction pool.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct PrioritizeVeto<T: Config + Send + Sync>(core::marker::PhantomData<T>);

impl<T: Config + Send + Sync> Default for PrioritizeVeto<T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<T: Config + Send + Sync> PrioritizeVeto<T> {
	pub fn new() -> Self {
		Self::default()
	}
}

impl<T: Config + Send + Sync> core::fmt::Debug for PrioritizeVeto<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "PrioritizeVeto")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
		Ok(())
	}
}

impl<T: Config + Send + Sync> TransactionExtension<T::RuntimeCall> for PrioritizeVeto<T>
where
	T::RuntimeCall: IsSubType<Call<T>>,
	T::AccountId: AsRef<[u8]>,
{
	const IDENTIFIER: &'static str = "PrioritizeVeto";
	type Implicit = ();
	type Val = ();
	type Pre = ();

	fn weight(&self, _: &T::RuntimeCall) -> Weight {
		Weight::zero()
	}

	fn validate(
		&self,
		origin: <T as polkadot_sdk::frame_system::Config>::RuntimeOrigin,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Encode,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, T::RuntimeCall> {
		let mut valid = ValidTransaction::default();
		let is_fisherman_call = matches!(
			call.is_sub_type(),
			Some(Call::veto_state_commitment { .. })
				| Some(Call::blacklist_dispute_game { .. })
				| Some(Call::blacklist_arbitrum_claim { .. })
		);
		if is_fisherman_call {
			if let Ok(account) = polkadot_sdk::frame_system::ensure_signed::<
				<T as polkadot_sdk::frame_system::Config>::RuntimeOrigin,
				T::AccountId,
			>(origin.clone())
			{
				if T::IsCollator::contains(&account) {
					valid.priority = TransactionPriority::MAX;
				}
			}
		}
		Ok((valid, (), origin))
	}

	impl_tx_ext_default!(T::RuntimeCall; prepare);
}
