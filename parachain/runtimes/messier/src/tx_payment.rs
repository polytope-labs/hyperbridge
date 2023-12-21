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

#![allow(dead_code)]

use alloc::vec::Vec;
use codec::{Decode, Encode};
use cumulus_primitives_core::Weight;
use frame_support::{
    dispatch::{CallableCallFor, DispatchClass, DispatchInfo, DispatchResult, PostDispatchInfo},
    pallet_prelude::TypeInfo,
    traits::{Currency, Defensive, IsSubType},
};
use ismp::handlers::handle_incoming_message;
use pallet_ismp::host::Host;
use pallet_transaction_payment::{ChargeTransactionPayment, Config, OnChargeTransaction, Pallet};
use sp_core::Get;
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, One, PostDispatchInfoOf, SignedExtension, Zero},
    transaction_validity::{
        TransactionPriority, TransactionValidity, TransactionValidityError, ValidTransaction,
    },
    SaturatedConversion, Saturating,
};
use std::ops::Mul;

/// Require the transactor pay for themselves and maybe include a tip to gain additional priority
/// in the queue. We've modified the code to allow users submit ISMP messages with valid proofs
/// without transaction fees.
///
/// # Transaction Validity
///
/// This extension sets the `priority` field of `TransactionValidity` depending on the amount
/// of tip being paid per weight unit.
///
/// Operational transactions will receive an additional priority bump, so that they are normally
/// considered before regular transactions.
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct IsmpTxPayment<T: Config>(#[codec(compact)] BalanceOf<T>);

impl<T: Config> IsmpTxPayment<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    BalanceOf<T>: Send + Sync,
{
    /// utility constructor. Used only in client/factory code.
    pub fn from(fee: BalanceOf<T>) -> Self {
        Self(fee)
    }

    /// Returns the tip as being chosen by the transaction sender.
    pub fn tip(&self) -> BalanceOf<T> {
        self.0
    }

    fn withdraw_fee(
        &self,
        who: &T::AccountId,
        call: &T::RuntimeCall,
        info: &DispatchInfoOf<T::RuntimeCall>,
        len: usize,
        must_succeed: bool,
    ) -> Result<
        Option<(
            BalanceOf<T>,
            <<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::LiquidityInfo,
        )>,
        TransactionValidityError,
    > {
        let tip = self.0;
        let fee = Pallet::<T>::compute_fee(len as u32, info, tip);

        let result = <<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::withdraw_fee(
            who, call, info, fee, tip,
        )
        .map(|i| (fee, i));

        match result {
            Ok(v) => Ok(Some(v)),
            Err(_) if !must_succeed => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Get an appropriate priority for a transaction with the given `DispatchInfo`, encoded length
    /// and user-included tip.
    ///
    /// The priority is based on the amount of `tip` the user is willing to pay per unit of either
    /// `weight` or `length`, depending which one is more limiting. For `Operational` extrinsics
    /// we add a "virtual tip" to the calculations.
    ///
    /// The formula should simply be `tip / bounded_{weight|length}`, but since we are using
    /// integer division, we have no guarantees it's going to give results in any reasonable
    /// range (might simply end up being zero). Hence we use a scaling factor:
    /// `tip * (max_block_{weight|length} / bounded_{weight|length})`, since given current
    /// state of-the-art blockchains, number of per-block transactions is expected to be in a
    /// range reasonable enough to not saturate the `Balance` type while multiplying by the tip.
    pub fn get_priority(
        info: &DispatchInfoOf<T::RuntimeCall>,
        len: usize,
        tip: BalanceOf<T>,
        final_fee: BalanceOf<T>,
    ) -> TransactionPriority {
        // Calculate how many such extrinsics we could fit into an empty block and take the
        // limiting factor.
        let max_block_weight = T::BlockWeights::get().max_block;
        let max_block_length = *T::BlockLength::get().max.get(info.class) as u64;

        // bounded_weight is used as a divisor later so we keep it non-zero.
        let bounded_weight = info.weight.max(Weight::from_parts(1, 1)).min(max_block_weight);
        let bounded_length = (len as u64).clamp(1, max_block_length);

        // returns the scarce resource, i.e. the one that is limiting the number of transactions.
        let max_tx_per_block_weight = max_block_weight
            .checked_div_per_component(&bounded_weight)
            .defensive_proof("bounded_weight is non-zero; qed")
            .unwrap_or(1);
        let max_tx_per_block_length = max_block_length / bounded_length;
        // Given our current knowledge this value is going to be in a reasonable range - i.e.
        // less than 10^9 (2^30), so multiplying by the `tip` value is unlikely to overflow the
        // balance type. We still use saturating ops obviously, but the point is to end up with some
        // `priority` distribution instead of having all transactions saturate the priority.
        let max_tx_per_block = max_tx_per_block_length
            .min(max_tx_per_block_weight)
            .saturated_into::<BalanceOf<T>>();
        let max_reward = |val: BalanceOf<T>| val.saturating_mul(max_tx_per_block);

        // To distribute no-tip transactions a little bit, we increase the tip value by one.
        // This means that given two transactions without a tip, smaller one will be preferred.
        let tip = tip.saturating_add(One::one());
        let scaled_tip = max_reward(tip);

        match info.class {
            DispatchClass::Normal => {
                // For normal class we simply take the `tip_per_weight`.
                scaled_tip
            },
            DispatchClass::Mandatory => {
                // Mandatory extrinsics should be prohibited (e.g. by the [`CheckWeight`]
                // extensions), but just to be safe let's return the same priority as `Normal` here.
                scaled_tip
            },
            DispatchClass::Operational => {
                // A "virtual tip" value added to an `Operational` extrinsic.
                // This value should be kept high enough to allow `Operational` extrinsics
                // to get in even during congestion period, but at the same time low
                // enough to prevent a possible spam attack by sending invalid operational
                // extrinsics which push away regular transactions from the pool.
                let fee_multiplier = T::OperationalFeeMultiplier::get().saturated_into();
                let virtual_tip = final_fee.saturating_mul(fee_multiplier);
                let scaled_virtual_tip = max_reward(virtual_tip);

                scaled_tip.saturating_add(scaled_virtual_tip)
            },
        }
        .saturated_into::<TransactionPriority>()
    }
}

type BalanceOf<T> = <<T as Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;

impl<T: Config> sp_std::fmt::Debug for IsmpTxPayment<T> {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "IsmpTxPayment<{:?}>", self.0)
    }
    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl<T> SignedExtension for IsmpTxPayment<T>
where
    T: Config + pallet_ismp::Config + pallet_balances::Config,
    BalanceOf<T>: Send + Sync + From<u64>,
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>
        + IsSubType<CallableCallFor<pallet_ismp::Pallet<T>, T>>, // for downcasting
{
    const IDENTIFIER: &'static str = "ChargeTransactionPayment";
    type AccountId = T::AccountId;
    type Call = T::RuntimeCall;
    type AdditionalSigned = ();
    type Pre = (
        // tip
        BalanceOf<T>,
        // who paid the fee - this is an option to allow for a Default impl.
        Self::AccountId,
        T::RuntimeCall,
    );
    fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        info: &DispatchInfoOf<Self::Call>,
        len: usize,
    ) -> TransactionValidity {
        let account = frame_system::Pallet::<T>::account(who);
        //
        if account.providers.is_zero() && account.sufficients.is_zero() {
            let _ = <pallet_balances::Pallet<T> as Currency<T::AccountId>>::deposit_creating(
                who,
                T::ExistentialDeposit::get().mul(10u32.into()),
            );
        }
        let final_fee = match call.is_sub_type() {
            Some(pallet_ismp::Call::handle { messages }) => {
                let host = Host::<T>::default();
                let result = messages
                    .iter()
                    .map(|msg| handle_incoming_message(&host, msg.clone()))
                    .collect::<Result<Vec<_>, _>>();

                if result.is_err() {
                    let (final_fee, _) = self
                        .withdraw_fee(who, call, info, len, true)?
                        .expect("must_succed is true, can\'t return None; qed");

                    final_fee
                } else {
                    0u32.into()
                }
            },
            _ => {
                let (final_fee, _) = self
                    .withdraw_fee(who, call, info, len, true)?
                    .expect("must_succed is true, can\'t return None; qed");

                final_fee
            },
        };
        let tip = self.0;
        Ok(ValidTransaction {
            priority: ChargeTransactionPayment::<T>::get_priority(info, len, tip, final_fee),
            ..Default::default()
        })
    }

    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        _call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        Ok((self.0, who.clone(), _call.clone()))
    }

    fn post_dispatch(
        maybe_pre: Option<Self::Pre>,
        info: &DispatchInfoOf<Self::Call>,
        post_info: &PostDispatchInfoOf<Self::Call>,
        len: usize,
        _result: &DispatchResult,
    ) -> Result<(), TransactionValidityError> {
        let result = maybe_pre.and_then(|(tip, who, call)| {
            IsmpTxPayment::<T>(tip)
                // doesn't need to succeed since we've checked in validate
                .withdraw_fee(&who, &call, info, len, false)
                .ok()
                .flatten()
                .map(|(_, imbalance)| (tip, who, imbalance))
        });

        if let Some((tip, who, imbalance)) = result {
            ChargeTransactionPayment::<T>::post_dispatch(
                Some((tip, who, imbalance)),
                info,
                post_info,
                len,
                _result,
            )?;
        }
        Ok(())
    }
}
