use codec::MaxEncodedLen;
use core::mem;
use scale_info::TypeInfo;
use polkadot_sdk::{
    frame_support::{
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
        traits::{MaybeSerializeDeserialize, Saturating, Zero},
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

pub struct AssetCurrencyAdapter<Assets, AssetId, B, AccountId, Reason>(
    PhantomData<(Assets, AssetId, B, AccountId, Reason)>,
);
impl<Assets, AssetId, B, AccountId, Reason> Currency<AccountId>
for AssetCurrencyAdapter<Assets, AssetId, B, AccountId, Reason>
where
    Assets: fungibles::Mutate<AccountId, AssetId = AssetId, Balance = B>
    + fungibles::Balanced<AccountId, AssetId = AssetId, Balance = B>
    + fungibles::Unbalanced<AccountId, AssetId = AssetId, Balance = B>,
    AssetId: Get<Assets::AssetId>,
    B: Balance + MaybeSerializeDeserialize + Debug + MaxEncodedLen,
    AccountId: Ord + Clone + Default + MaxEncodedLen + TypeInfo,
    Reason: Default,
{
    type Balance = B;
    type PositiveImbalance = PositiveImbalance<Self::Balance>;
    type NegativeImbalance = NegativeImbalance<Self::Balance>;

    fn total_balance(who: &AccountId) -> Self::Balance {
        Assets::balance(AssetId::get(), who)
    }

    fn can_slash(who: &AccountId, value: Self::Balance) -> bool {
        Self::total_balance(who) >= value
    }

    fn total_issuance() -> Self::Balance {
        Assets::total_issuance(AssetId::get())
    }

    fn minimum_balance() -> Self::Balance {
        Assets::minimum_balance(AssetId::get())
    }

    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        let _ = Assets::burn_from(
            AssetId::get(),
            &AccountId::default(),
            amount,
            Preservation::Expendable,
            Precision::Exact,
            Fortitude::Polite,
        );
        PositiveImbalance::new(amount)
    }

    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        let _ = Assets::mint_into(AssetId::get(), &AccountId::default(), amount);
        NegativeImbalance::new(amount)
    }

    fn free_balance(who: &AccountId) -> Self::Balance {
        Assets::balance(AssetId::get(), who)
    }

    fn ensure_can_withdraw(
        who: &AccountId,
        amount: Self::Balance,
        _reasons: WithdrawReasons,
        new_balance: Self::Balance,
    ) -> SpDispatchResult {
        Assets::can_withdraw(AssetId::get(), who, amount).into_result(false)?;
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
        Assets::transfer(AssetId::get(), source, dest, value, preservation)?;
        Ok(())
    }

    fn slash(who: &AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        let (imbalance, remaining) = Assets::slash(AssetId::get(), who, value);
        (imbalance, remaining)
    }

    fn deposit_into_existing(
        who: &AccountId,
        value: Self::Balance,
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        Assets::mint_into(AssetId::get(), who, value).map(PositiveImbalance::new)
    }

    fn deposit_creating(who: &AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        PositiveImbalance::new(
            Assets::mint_into(AssetId::get(), who, value).unwrap_or_else(|_| B::zero()),
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
        Assets::burn_from(
            AssetId::get(),
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

impl<Assets, AssetId, B, AccountId, Reason> ReservableCurrency<AccountId>
for AssetCurrencyAdapter<Assets, AssetId, B, AccountId, Reason>
where
    Assets: fungibles::Mutate<AccountId, AssetId = AssetId, Balance = B>
    + fungibles::Balanced<AccountId, AssetId = AssetId, Balance = B>
    + fungibles::Unbalanced<AccountId, AssetId = AssetId, Balance = B>
    + fungibles::hold::Mutate<AccountId, AssetId = AssetId, Balance = B, Reason = Reason>
    + fungibles::hold::Unbalanced<AccountId, AssetId = AssetId, Balance = B, Reason = Reason>
    + fungibles::BalancedHold<AccountId, AssetId = AssetId, Balance = B, Reason = Reason>,
    AssetId: Get<Assets::AssetId>,
    B: Balance + MaybeSerializeDeserialize + Debug + MaxEncodedLen,
    AccountId: Ord + Clone + Default + MaxEncodedLen + TypeInfo,
    Reason: codec::Codec + Clone + PartialEq + Eq + MaxEncodedLen + TypeInfo + Default,
{
    fn can_reserve(who: &AccountId, value: Self::Balance) -> bool {
        Assets::can_hold(AssetId::get(), &Reason::default(), who, value)
    }

    fn reserved_balance(who: &AccountId) -> Self::Balance {
        Assets::balance_on_hold(AssetId::get(), &Reason::default(), who)
    }

    fn reserve(who: &AccountId, value: Self::Balance) -> SpDispatchResult {
        Assets::hold(AssetId::get(), &Reason::default(), who, value)?;
        Ok(())
    }

    fn unreserve(who: &AccountId, value: Self::Balance) -> Self::Balance {
        Assets::release(AssetId::get(), &Reason::default(), who, value, Precision::Exact)
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
        Assets::transfer_on_hold(
            AssetId::get(),
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
        let (imbalance, remaining) =
            <Assets as fungibles::hold::Unbalanced<AccountId>>::slash_on_hold(AssetId::get(), &Reason::default(), who, value);
        (imbalance, remaining)
    }
}
