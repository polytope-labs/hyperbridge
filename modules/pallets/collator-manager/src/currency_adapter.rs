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

use codec::MaxEncodedLen;
use polkadot_sdk::{
	frame_support::traits::{
		Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons,
		fungible::{Credit, Debt},
		tokens::{
			Balance, BalanceStatus, Fortitude, Precision, Preservation, Restriction, fungible,
			imbalance::SignedImbalance,
		},
	},
	sp_runtime::{
		DispatchError, DispatchResult as SpDispatchResult, traits::MaybeSerializeDeserialize,
	},
	sp_std::{fmt::Debug, marker::PhantomData},
};
use scale_info::TypeInfo;

pub struct FungibleToCurrencyAdapter<F, H, B, AccountId, Reason>(
	PhantomData<(F, H, B, AccountId, Reason)>,
);

impl<F, H, B, AccountId, Reason> Currency<AccountId>
	for FungibleToCurrencyAdapter<F, H, B, AccountId, Reason>
where
	F: fungible::Mutate<AccountId, Balance = B> + fungible::Balanced<AccountId, Balance = B>,
	H: fungible::hold::Mutate<AccountId, Reason = Reason, Balance = B>,
	B: Balance + MaybeSerializeDeserialize + Debug + MaxEncodedLen,
	AccountId: Ord + Clone + MaxEncodedLen + TypeInfo,
	Reason: Default,
{
	type Balance = B;
	type PositiveImbalance = Debt<AccountId, F>;
	type NegativeImbalance = Credit<AccountId, F>;

	fn total_balance(who: &AccountId) -> Self::Balance {
		F::total_balance(who)
	}

	fn can_slash(who: &AccountId, value: Self::Balance) -> bool {
		F::can_withdraw(who, value).into_result(false).is_ok()
	}

	fn total_issuance() -> Self::Balance {
		F::total_issuance()
	}

	fn minimum_balance() -> Self::Balance {
		F::minimum_balance()
	}

	fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
		F::rescind(amount)
	}

	fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
		F::issue(amount)
	}

	fn free_balance(who: &AccountId) -> Self::Balance {
		F::balance(who)
	}

	fn ensure_can_withdraw(
		who: &AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
		new_balance: Self::Balance,
	) -> SpDispatchResult {
		F::can_withdraw(who, amount).into_result(false)?;
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
		F::transfer(source, dest, value, preservation)?;
		Ok(())
	}

	fn slash(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		let available = F::reducible_balance(who, Preservation::Expendable, Fortitude::Polite);
		let slash_amount = value.min(available);
		let burned = F::burn_from(
			who,
			slash_amount,
			Preservation::Expendable,
			Precision::BestEffort,
			Fortitude::Force,
		)
		.unwrap_or_else(|_| B::zero());
		(F::issue(burned), value.saturating_sub(burned))
	}

	fn deposit_into_existing(
		who: &AccountId,
		value: Self::Balance,
	) -> Result<Self::PositiveImbalance, DispatchError> {
		F::deposit(who, value, Precision::Exact)
	}

	fn deposit_creating(who: &AccountId, value: Self::Balance) -> Self::PositiveImbalance {
		F::deposit(who, value, Precision::Exact).unwrap_or_else(|_| F::rescind(B::zero()))
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
		F::withdraw(who, value, Precision::Exact, preservation, Fortitude::Polite)
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
				SignedImbalance::Positive(F::rescind(B::zero()))
			}
		} else if current_balance > balance {
			let diff = current_balance.saturating_sub(balance);
			if let Ok(imb) =
				Self::withdraw(who, diff, WithdrawReasons::all(), ExistenceRequirement::AllowDeath)
			{
				SignedImbalance::Negative(imb)
			} else {
				SignedImbalance::Negative(F::issue(B::zero()))
			}
		} else {
			SignedImbalance::Positive(F::rescind(B::zero()))
		}
	}
}

impl<F, H, B, AccountId, Reason> ReservableCurrency<AccountId>
	for FungibleToCurrencyAdapter<F, H, B, AccountId, Reason>
where
	F: fungible::Mutate<AccountId, Balance = B> + fungible::Balanced<AccountId, Balance = B>,
	H: fungible::hold::Mutate<AccountId, Reason = Reason, Balance = B>,
	B: Balance + MaybeSerializeDeserialize + Debug + MaxEncodedLen,
	AccountId: Ord + Clone + MaxEncodedLen + TypeInfo,
	Reason: codec::Codec + Clone + PartialEq + Eq + MaxEncodedLen + TypeInfo + Default,
{
	fn can_reserve(who: &AccountId, value: Self::Balance) -> bool {
		H::can_hold(&Reason::default(), who, value)
	}

	fn reserved_balance(who: &AccountId) -> Self::Balance {
		H::balance_on_hold(&Reason::default(), who)
	}

	fn reserve(who: &AccountId, value: Self::Balance) -> SpDispatchResult {
		H::hold(&Reason::default(), who, value)?;
		Ok(())
	}

	fn unreserve(who: &AccountId, value: Self::Balance) -> Self::Balance {
		H::release(&Reason::default(), who, value, Precision::Exact).unwrap_or_default()
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
		H::transfer_on_hold(
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
		let released = H::release(&Reason::default(), who, slash_amount, Precision::BestEffort)
			.unwrap_or_else(|_| B::zero());
		let burned = F::burn_from(
			who,
			released,
			Preservation::Expendable,
			Precision::BestEffort,
			Fortitude::Force,
		)
		.unwrap_or_else(|_| B::zero());
		(F::issue(burned), value.saturating_sub(burned))
	}
}
