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

use codec::{Decode, MaxEncodedLen};
use core::mem;
use scale_info::TypeInfo;
use polkadot_sdk::{
    frame_support::{
        PalletId,
        traits::{
            fungibles,
            tokens::{
                Balance,
                imbalance::{Imbalance as ImbalanceT, SignedImbalance, TryMerge},
                BalanceStatus, Fortitude, Precision, Preservation, Restriction,
            },
            Currency, ExistenceRequirement, Get, ReservableCurrency, TryDrop, WithdrawReasons,
        },
    },
    sp_runtime::{
        traits::{MaybeSerializeDeserialize, AccountIdConversion},
        DispatchError, DispatchResult as SpDispatchResult,
    },
    sp_std::{fmt::Debug, marker::PhantomData, result},
};
use polkadot_sdk::frame_support::traits::SameOrOther;

pub struct PositiveImbalance<B: Balance>(B);
impl<B: Balance> PositiveImbalance<B> {
    pub fn new(amount: B) -> Self {
        Self(amount)
    }
}

impl<B: Balance> TryDrop for PositiveImbalance<B> {
    fn try_drop(self) -> result::Result<(), Self> {
        if self.0.is_zero() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

impl<B: Balance> Default for PositiveImbalance<B> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<B: Balance> TryMerge for PositiveImbalance<B> {
    fn try_merge(self, other: Self) -> Result<Self, (Self, Self)> {
        Ok(self.merge(other))
    }
}

impl<B: Balance> ImbalanceT<B> for PositiveImbalance<B> {
    type Opposite = NegativeImbalance<B>;

    fn zero() -> Self {
        Self(B::zero())
    }
    fn peek(&self) -> B {
        self.0
    }
    fn drop_zero(self) -> result::Result<(), Self> {
        self.try_drop()
    }
    fn split(self, amount: B) -> (Self, Self) {
        let first = self.0.min(amount);
        let second = self.0 - first;
        mem::forget(self);
        (Self(first), Self(second))
    }
    fn extract(&mut self, amount: B) -> Self {
        let new = self.0.min(amount);
        self.0 = self.0 - new;
        Self(new)
    }
    fn merge(mut self, other: Self) -> Self {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
        self
    }
    fn subsume(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
    }
    fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
        let me = self.0;
        let them = other.0;
        mem::forget(self);
        mem::forget(other);
        if me > them {
            SameOrOther::Same(Self::new(me - them))
        } else if them > me {
            SameOrOther::Other(NegativeImbalance::new(them - me))
        } else {
            SameOrOther::None
        }
    }
}

pub struct NegativeImbalance<B: Balance>(B);
impl<B: Balance> NegativeImbalance<B> {
    pub fn new(amount: B) -> Self {
        Self(amount)
    }
}

impl<B: Balance> TryDrop for NegativeImbalance<B> {
    fn try_drop(self) -> result::Result<(), Self> {
        if self.0.is_zero() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

impl<B: Balance> Default for NegativeImbalance<B> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<B: Balance> TryMerge for NegativeImbalance<B> {
    fn try_merge(self, other: Self) -> Result<Self, (Self, Self)> {
        Ok(self.merge(other))
    }
}

impl<B: Balance> ImbalanceT<B> for NegativeImbalance<B> {
    type Opposite = PositiveImbalance<B>;

    fn zero() -> Self {
        Self(B::zero())
    }
    fn peek(&self) -> B {
        self.0
    }
    fn drop_zero(self) -> result::Result<(), Self> {
        self.try_drop()
    }
    fn split(self, amount: B) -> (Self, Self) {
        let first = self.0.min(amount);
        let second = self.0 - first;
        mem::forget(self);
        (Self(first), Self(second))
    }
    fn extract(&mut self, amount: B) -> Self {
        let new = self.0.min(amount);
        self.0 = self.0 - new;
        Self(new)
    }
    fn merge(mut self, other: Self) -> Self {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
        self
    }
    fn subsume(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
    }
    fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
        let me = self.0;
        let them = other.0;
        mem::forget(self);
        mem::forget(other);
        if me > them {
            SameOrOther::Same(Self::new(me - them))
        } else if them > me {
            SameOrOther::Other(PositiveImbalance::new(them - me))
        } else {
            SameOrOther::None
        }
    }
}

pub struct AssetCurrencyAdapter<Base, Holder, AssetIdParameter, B, AccountId, PotId, Reason>(
    PhantomData<(Base, Holder, AssetIdParameter, B, AccountId, PotId, Reason)>,
);
impl<Base, Holder, AssetIdParameter, B, AccountId, PotId, Reason> Currency<AccountId>
for AssetCurrencyAdapter<Base, Holder, AssetIdParameter, B, AccountId, PotId, Reason>
where
    Base: fungibles::Mutate<AccountId, Balance = B>
    + fungibles::Balanced<AccountId, Balance = B>,
    AssetIdParameter: Get<Base::AssetId>,
    B: Balance + MaybeSerializeDeserialize + Debug + MaxEncodedLen,
    AccountId: Ord + Clone + MaxEncodedLen + TypeInfo + Decode,
    PotId: Get<PalletId>,
    Reason: Default,
{
    type Balance = B;
    type PositiveImbalance = PositiveImbalance<Self::Balance>;
    type NegativeImbalance = NegativeImbalance<Self::Balance>;

    fn total_balance(who: &AccountId) -> Self::Balance {
        Base::balance(AssetIdParameter::get(), who)
    }

    fn can_slash(who: &AccountId, value: Self::Balance) -> bool {
        Self::total_balance(who) >= value
    }

    fn total_issuance() -> Self::Balance {
        Base::total_issuance(AssetIdParameter::get())
    }

    fn minimum_balance() -> Self::Balance {
        Base::minimum_balance(AssetIdParameter::get())
    }

    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        let pot = PotId::get().into_account_truncating();
        let burned = Base::burn_from(
            AssetIdParameter::get(),
            &pot,
            amount,
            Preservation::Expendable,
            Precision::Exact,
            Fortitude::Polite,
        ).unwrap_or_else(|_| B::zero());
        PositiveImbalance::new(burned)
    }

    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        let pot = PotId::get().into_account_truncating();
        let issued = Base::mint_into(AssetIdParameter::get(), &pot, amount).unwrap_or_else(|_| B::zero());
        NegativeImbalance::new(issued)
    }

    fn free_balance(who: &AccountId) -> Self::Balance {
        Base::balance(AssetIdParameter::get(), who)
    }

    fn ensure_can_withdraw(
        who: &AccountId,
        amount: Self::Balance,
        _reasons: WithdrawReasons,
        new_balance: Self::Balance,
    ) -> SpDispatchResult {
        Base::can_withdraw(AssetIdParameter::get(), who, amount).into_result(false)?;
        if new_balance < Self::minimum_balance() {
            return Err(DispatchError::Other("Would fall below minimum balance"));
        }
        Ok(())
    }

    fn transfer(
        source: &AccountId,
        dest: &AccountId,
        value: Self::Balance,
        existence_requirement: ExistenceRequirement,
    ) -> SpDispatchResult {
        let preservation = match existence_requirement {
            ExistenceRequirement::KeepAlive => Preservation::Protect,
            ExistenceRequirement::AllowDeath => Preservation::Expendable,
        };
        Base::transfer(AssetIdParameter::get(), source, dest, value, preservation)?;
        Ok(())
    }

    fn slash(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        let available = Base::reducible_balance(AssetIdParameter::get(), who, Preservation::Expendable, Fortitude::Polite);
        let slash_amount = value.min(available);
        let burned = Base::burn_from(AssetIdParameter::get(), who, slash_amount, Preservation::Expendable, Precision::BestEffort, Fortitude::Force).unwrap_or_else(|_| B::zero());
        (NegativeImbalance::new(burned), value.saturating_sub(burned))
    }

    fn deposit_into_existing(
        who: &AccountId,
        value: Self::Balance,
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        Base::mint_into(AssetIdParameter::get(), who, value).map(PositiveImbalance::new)
    }

    fn deposit_creating(who: &AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        PositiveImbalance::new(
            Base::mint_into(AssetIdParameter::get(), who, value).unwrap_or_else(|_| B::zero()),
        )
    }

    fn withdraw(
        who: &AccountId,
        value: Self::Balance,
        _reasons: WithdrawReasons,
        liveness: ExistenceRequirement,
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        let preservation = match liveness {
            ExistenceRequirement::KeepAlive => Preservation::Protect,
            ExistenceRequirement::AllowDeath => Preservation::Expendable,
        };
        Base::burn_from(
            AssetIdParameter::get(),
            who,
            value,
            preservation,
            Precision::Exact,
            Fortitude::Polite,
        )
            .map(NegativeImbalance::new)
    }

    fn make_free_balance_be(
        who: &AccountId,
        balance: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        let current_balance = Self::free_balance(who);
        if current_balance < balance {
            let diff = balance.saturating_sub(current_balance);
            if let Ok(imb) = Self::deposit_into_existing(who, diff) {
                SignedImbalance::Positive(imb)
            } else {
                SignedImbalance::Positive(PositiveImbalance::new(B::zero()))
            }
        } else if current_balance > balance {
            let diff = current_balance.saturating_sub(balance);
            if let Ok(imb) =
                Self::withdraw(who, diff, WithdrawReasons::all(), ExistenceRequirement::AllowDeath)
            {
                SignedImbalance::Negative(imb)
            } else {
                SignedImbalance::Negative(NegativeImbalance::new(B::zero()))
            }
        } else {
            SignedImbalance::Positive(PositiveImbalance::new(B::zero()))
        }
    }
}

impl<Base, Holder, AssetIdParameter, B, AccountId, PotId, Reason> ReservableCurrency<AccountId>
for AssetCurrencyAdapter<Base, Holder, AssetIdParameter, B, AccountId, PotId, Reason>
where
    Base: fungibles::Mutate<AccountId, Balance = B>
    + fungibles::Balanced<AccountId, Balance = B>,
    Holder: fungibles::hold::Mutate<AccountId, Reason = Reason, Balance = B>
    + fungibles::hold::Inspect<AccountId, Reason = Reason, Balance = B>,
    AssetIdParameter: Get<Base::AssetId> + Get<Holder::AssetId>,
    B: Balance + MaybeSerializeDeserialize + Debug + MaxEncodedLen,
    AccountId: Ord + Clone + MaxEncodedLen + TypeInfo + Decode,
    PotId: Get<PalletId>,
    Reason: codec::Codec + Clone + PartialEq + Eq + MaxEncodedLen + TypeInfo + Default,
{
    fn can_reserve(who: &AccountId, value: Self::Balance) -> bool {
        Holder::can_hold(AssetIdParameter::get(), &Reason::default(), who, value)
    }

    fn reserved_balance(who: &AccountId) -> Self::Balance {
        Holder::balance_on_hold(AssetIdParameter::get(), &Reason::default(), who)
    }

    fn reserve(who: &AccountId, value: Self::Balance) -> SpDispatchResult {
        Holder::hold(AssetIdParameter::get(), &Reason::default(), who, value)?;
        Ok(())
    }

    fn unreserve(who: &AccountId, value: Self::Balance) -> Self::Balance {
        Holder::release(AssetIdParameter::get(), &Reason::default(), who, value, Precision::Exact)
            .unwrap_or_default()
    }

    fn repatriate_reserved(
        slashed: &AccountId,
        beneficiary: &AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> Result<B, DispatchError> {
        let restriction = match status {
            BalanceStatus::Free => Restriction::Free,
            BalanceStatus::Reserved => Restriction::OnHold,
        };
        Holder::transfer_on_hold(
            AssetIdParameter::get(),
            &Reason::default(),
            slashed,
            beneficiary,
            value,
            Precision::Exact,
            restriction,
            Fortitude::Polite,
        )
    }

    fn slash_reserved(
        who: &AccountId,
        value: Self::Balance,
    ) -> (Self::NegativeImbalance, Self::Balance) {
        let slash_amount = value.min(Self::reserved_balance(who));
        let released = Holder::release(AssetIdParameter::get(), &Reason::default(), who, slash_amount, Precision::BestEffort).unwrap_or_else(|_| B::zero());
        let burned = Base::burn_from(AssetIdParameter::get(), who, released, Preservation::Expendable, Precision::BestEffort, Fortitude::Force).unwrap_or_else(|_| B::zero());
        (NegativeImbalance::new(burned), value.saturating_sub(burned))
    }
}
