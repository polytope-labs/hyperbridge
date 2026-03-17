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

#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod benchmarking;
#[cfg(test)]
mod tests;
pub mod types;
mod weights;

use alloc::vec::Vec;
use codec::Encode as _;
use frame_support::{
	ensure,
	traits::{Currency, ReservableCurrency},
	BoundedVec,
};
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	host::{IsmpHost, StateMachine},
};
use polkadot_sdk::*;
use primitive_types::{H160, H256};
use sp_core::Get;
use sp_io::offchain_index;
use sp_runtime::{
	traits::{ConstU32, Zero},
	Saturating,
};
pub use weights::WeightInfo;

use types::{
	Bid, GatewayInfo, IntentGatewayParams, PriceEntry, PriceInput, RequestKind,
	TokenDecimalsUpdate, TokenInfo, TokenPair,
};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// Pallet identifier for ISMP routing
pub const PALLET_INTENTS_ID: &[u8] = b"pallet-intents";

/// Generate the offchain storage key for a bid given raw commitment and filler bytes.
pub fn offchain_bid_key_raw(commitment: &H256, filler_encoded: &[u8]) -> Vec<u8> {
	let mut key = b"intents::bid::".to_vec();
	key.extend_from_slice(commitment.as_bytes());
	key.extend_from_slice(filler_encoded);
	key
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::alloc::string::ToString;
	use alloc::vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The [`IsmpDispatcher`] for dispatching cross-chain requests
		type Dispatcher: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>
			+ ismp::host::IsmpHost;

		/// A currency implementation for handling storage deposits
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The default storage deposit fee per bid (used as fallback)
		#[pallet::constant]
		type StorageDepositFee: Get<BalanceOf<Self>>;

		/// Origin that can perform governance actions
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Maximum number of price entries per submission
		#[pallet::constant]
		type MaxPriceEntries: Get<u32>;

		/// Weight information for extrinsics in this pallet
		type WeightInfo: WeightInfo;
	}

	/// Type alias for the balance type
	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as polkadot_sdk::frame_system::Config>::AccountId,
	>>::Balance;

	/// Storage for bids indexed by commitment and filler address
	/// Allows easy discovery of all bids for a given order commitment
	/// The actual bid data is stored in offchain storage
	/// We store the deposit amount here for accurate refunds
	#[pallet::storage]
	pub type Bids<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		H256, // commitment
		Blake2_128Concat,
		T::AccountId, // filler
		BalanceOf<T>, // deposit amount, actual bid data in offchain storage
		OptionQuery,
	>;

	/// The storage deposit fee per bid, updatable via governance
	#[pallet::storage]
	pub type StorageDepositFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Storage for Intent Gateway deployments per state machine
	#[pallet::storage]
	pub type Gateways<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachine, GatewayInfo, OptionQuery>;

	/// Recognized token pairs for price tracking
	#[pallet::storage]
	pub type RecognizedPairs<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, TokenPair, OptionQuery>;

	/// Start timestamp (in seconds) of the current price window
	#[pallet::storage]
	pub type PriceWindowStart<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Price window duration in milliseconds
	#[pallet::storage]
	pub type PriceWindowDurationValue<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Price entries per pair
	#[pallet::storage]
	pub type Prices<T: Config> = StorageMap<_, Blake2_128Concat, H256, Vec<PriceEntry>, ValueQuery>;

	/// Deposits reserved by price submitters. Maps (account, pair_id) to
	/// (deposit_amount, unlock_block). When `unlock_block` is `None`, the withdrawal
	/// has not been initiated. The first call to `withdraw_price_deposit` sets
	/// `unlock_block` to `current_block + PriceDepositLockDuration`. The second
	/// call (after that block) unreserves the tokens.
	#[pallet::storage]
	pub type PriceDeposits<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		H256,                                      // pair_id
		(BalanceOf<T>, Option<BlockNumberFor<T>>), // (deposit_amount, unlock_block)
		OptionQuery,
	>;

	/// The amount reserved from submitters on their first price submission per pair
	#[pallet::storage]
	pub type PriceDepositAmount<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// How many blocks the price deposit is locked before it can be withdrawn.
	/// When a filler initiates a withdrawal, the unlock block is set to
	/// `current_block + PriceDepositLockDuration`.
	#[pallet::storage]
	pub type PriceDepositLockDuration<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Whether prices have been cleared in the current window.
	/// Reset to false by `on_initialize` when a new window starts.
	/// Set to true on the first price submission in the new window.
	#[pallet::storage]
	pub type PricesClearedThisWindow<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let now = T::Dispatcher::default().timestamp().as_secs();
			let window_duration_secs = PriceWindowDurationValue::<T>::get().saturating_div(1000);

			// Nothing to do if duration is not configured
			if window_duration_secs == 0 {
				return T::DbWeight::get().reads(2);
			}

			let window_start = PriceWindowStart::<T>::get();

			if window_start == 0 || now.saturating_sub(window_start) >= window_duration_secs {
				PriceWindowStart::<T>::put(now);
				PricesClearedThisWindow::<T>::put(false);

				T::DbWeight::get().reads(3).saturating_add(T::DbWeight::get().writes(2))
			} else {
				T::DbWeight::get().reads(3)
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A bid was placed by a filler
		BidPlaced { filler: T::AccountId, commitment: H256, deposit: BalanceOf<T> },
		/// A bid was retracted by a filler
		BidRetracted { filler: T::AccountId, commitment: H256, refund: BalanceOf<T> },
		/// Gateway parameters were updated
		GatewayParamsUpdated {
			state_machine: StateMachine,
			old_params: IntentGatewayParams,
			new_params: IntentGatewayParams,
		},
		/// New Intent Gateway deployment was added
		GatewayDeploymentAdded { state_machine: StateMachine, gateway: H160 },
		/// Dust sweep was initiated
		DustSweepInitiated {
			state_machine: StateMachine,
			beneficiary: H160,
			tokens: Vec<TokenInfo>,
		},
		/// Token decimals update was initiated
		TokenDecimalsUpdateInitiated {
			state_machine: StateMachine,
			updates: Vec<TokenDecimalsUpdate>,
		},
		/// Storage deposit fee was updated
		StorageDepositFeeUpdated { fee: BalanceOf<T> },
		/// A recognized token pair was added
		RecognizedPairAdded { pair_id: H256, pair: TokenPair },
		/// A recognized token pair was removed
		RecognizedPairRemoved { pair_id: H256 },
		/// Prices were submitted for a token pair
		PriceSubmitted { submitter: T::AccountId, pair_id: H256 },
		/// Price window duration was updated
		PriceWindowDurationUpdated { duration_ms: u64 },
		/// Price deposit amount was updated
		PriceDepositAmountUpdated { amount: BalanceOf<T> },
		/// Price deposit lock duration was updated (in blocks)
		PriceDepositLockDurationUpdated { duration_blocks: BlockNumberFor<T> },
		/// Price deposit was reserved on first submission
		PriceDepositReserved { submitter: T::AccountId, pair_id: H256, amount: BalanceOf<T> },
		/// Price deposit withdrawal was initiated (unlock block noted)
		PriceDepositWithdrawalInitiated {
			submitter: T::AccountId,
			pair_id: H256,
			unlock_block: BlockNumberFor<T>,
		},
		/// Price deposit was withdrawn (tokens unreserved)
		PriceDepositWithdrawn { submitter: T::AccountId, pair_id: H256, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The bid does not exist
		BidNotFound,
		/// The caller is not the filler who placed the bid
		NotBidOwner,
		/// Insufficient balance to pay storage deposit
		InsufficientBalance,
		/// Gateway not found for the specified state machine
		GatewayNotFound,
		/// Invalid user operation data
		InvalidUserOp,
		/// Failed to dispatch cross-chain request
		DispatchFailed,
		/// Token pair not recognized
		PairNotRecognized,
		/// Token pair already exists
		PairAlreadyExists,
		/// The price range is invalid (range_start > range_end)
		InvalidPriceRange,
		/// No price entries were provided
		EmptyPriceEntries,
		/// Price deposits are not configured (amount is zero)
		PriceDepositsNotConfigured,
		/// No deposit found for this account and pair
		DepositNotFound,
		/// The deposit is still within the lock duration (unlock block not yet reached)
		DepositStillLocked,
		/// Cannot submit prices while withdrawal is pending
		WithdrawalInProgress,
		/// Withdrawal has already been initiated
		WithdrawalAlreadyInitiated,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		/// Place a bid for an order
		///
		/// # Parameters
		/// - `commitment`: The order commitment hash
		/// - `user_op`: The signed user operation as opaque bytes (max 1MB)
		///
		/// # Errors
		/// - `InsufficientBalance`: If the filler doesn't have enough balance for the deposit
		/// - `InvalidUserOp`: If the user operation data is invalid or exceeds 1MB
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::place_bid())]
		pub fn place_bid(
			origin: OriginFor<T>,
			commitment: H256,
			user_op: BoundedVec<u8, ConstU32<1_048_576>>,
		) -> DispatchResult {
			let filler = ensure_signed(origin)?;

			// Validate user_op is not empty
			ensure!(!user_op.is_empty(), Error::<T>::InvalidUserOp);

			// If a bid already exists, unreserve the old deposit first
			if let Some(old_deposit) = Bids::<T>::get(&commitment, &filler) {
				<T as Config>::Currency::unreserve(&filler, old_deposit);
			}

			let deposit = Self::storage_deposit_fee();

			// Reserve the new deposit
			<T as Config>::Currency::reserve(&filler, deposit)
				.map_err(|_| Error::<T>::InsufficientBalance)?;

			// Store the bid in offchain storage
			let bid = Bid { filler: filler.clone(), user_op: user_op.to_vec() };
			let offchain_key = Self::offchain_bid_key(&commitment, &filler);
			offchain_index::set(&offchain_key, &bid.encode());

			// Store deposit amount in onchain storage for discoverability and accurate refunds
			Bids::<T>::insert(&commitment, &filler, deposit);

			Self::deposit_event(Event::BidPlaced { filler, commitment, deposit });

			Ok(())
		}

		/// Retract a bid and receive deposit refund
		///
		/// # Parameters
		/// - `commitment`: The order commitment hash
		///
		/// # Errors
		/// - `BidNotFound`: If no bid exists for this filler and commitment
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::retract_bid())]
		pub fn retract_bid(origin: OriginFor<T>, commitment: H256) -> DispatchResult {
			let filler = ensure_signed(origin)?;

			// Get the bid deposit amount
			let deposit = Bids::<T>::get(&commitment, &filler).ok_or(Error::<T>::BidNotFound)?;

			// Unreserve the deposit
			<T as Config>::Currency::unreserve(&filler, deposit);

			// Remove the bid marker from onchain storage
			Bids::<T>::remove(&commitment, &filler);

			// Clear the bid from offchain storage
			let offchain_key = Self::offchain_bid_key(&commitment, &filler);
			offchain_index::clear(&offchain_key);

			Self::deposit_event(Event::BidRetracted { filler, commitment, refund: deposit });

			Ok(())
		}

		/// Add a new Intent Gateway deployment
		///
		/// # Parameters
		/// - `state_machine`: The state machine identifier
		/// - `gateway`: The gateway contract address
		/// - `params`: Initial parameters for the gateway
		///
		/// # Errors
		/// - `GatewayNotFound`: If the gateway doesn't exist for the specified state machine
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::add_deployment())]
		pub fn add_deployment(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			gateway: H160,
			params: IntentGatewayParams,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Store gateway info
			let gateway_info = GatewayInfo { gateway, params };

			Gateways::<T>::insert(state_machine, gateway_info);

			// Notify all existing gateways about the new deployment
			// Only notify gateways with different addresses (same address automatically accepts)
			for (existing_state_machine, existing_gateway_info) in Gateways::<T>::iter() {
				// Skip if same state machine or same gateway address
				if existing_state_machine == state_machine ||
					existing_gateway_info.gateway == gateway
				{
					continue;
				}

				// Prepare cross-chain request to notify existing gateway
				let new_deployment = types::NewDeployment {
					state_machine_id: state_machine.to_string().into_bytes(),
					gateway,
				};
				let request = RequestKind::AddDeployment(new_deployment);
				let body = request.encode_body();

				// Dispatch cross-chain message (ignore errors to not fail the whole operation)
				let _ = Self::dispatch(existing_state_machine, existing_gateway_info.gateway, body);
			}

			Self::deposit_event(Event::GatewayDeploymentAdded { state_machine, gateway });

			Ok(())
		}

		/// Update Intent Gateway parameters via cross-chain governance
		///
		/// # Parameters
		/// - `state_machine`: The state machine where the gateway is deployed
		/// - `params_update`: The new parameters to apply
		/// - `fee`: Metadata for paying dispatch fees
		///
		/// # Errors
		/// - `GatewayNotFound`: If no gateway exists for the state machine
		/// - `DispatchFailed`: If cross-chain dispatch fails
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_params())]
		pub fn update_params(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			params_update: types::ParamsUpdate,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Get gateway info
			let mut gateway_info =
				Gateways::<T>::get(state_machine).ok_or(Error::<T>::GatewayNotFound)?;

			// Store old params for event
			let old_params = gateway_info.params.clone();

			// Merge update with current params
			let updated_params = gateway_info.params.update(params_update.clone());

			// Create complete params update for cross-chain dispatch
			let complete_update = types::CompleteParamsUpdate {
				params: updated_params.clone(),
				destination_fees: params_update.destination_fees.unwrap_or_default(),
			};

			// Prepare cross-chain request
			let request = RequestKind::UpdateParams(complete_update);
			let body = request.encode_body();

			// Dispatch cross-chain message
			Self::dispatch(state_machine, gateway_info.gateway, body)?;

			// Update local storage
			gateway_info.params = updated_params.clone();
			Gateways::<T>::insert(state_machine, gateway_info);

			Self::deposit_event(Event::GatewayParamsUpdated {
				state_machine,
				old_params,
				new_params: updated_params,
			});

			Ok(())
		}

		/// Sweep dust from an Intent Gateway
		///
		/// # Parameters
		/// - `state_machine`: The state machine where the gateway is deployed
		/// - `sweep_dust`: The sweep dust request
		/// - `fee`: Metadata for paying dispatch fees
		///
		/// # Errors
		/// - `GatewayNotFound`: If no gateway exists for the state machine
		/// - `DispatchFailed`: If cross-chain dispatch fails
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::sweep_dust())]
		pub fn sweep_dust(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			sweep_dust: types::SweepDust,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Get gateway info
			let gateway_info =
				Gateways::<T>::get(state_machine).ok_or(Error::<T>::GatewayNotFound)?;

			// Prepare cross-chain request
			let request = RequestKind::SweepDust(sweep_dust.clone());
			let body = request.encode_body();

			// Dispatch cross-chain message
			Self::dispatch(state_machine, gateway_info.gateway, body)?;

			Self::deposit_event(Event::DustSweepInitiated {
				state_machine,
				beneficiary: sweep_dust.beneficiary,
				tokens: sweep_dust.outputs,
			});

			Ok(())
		}

		/// Update token decimals in VWAP Oracle via cross-chain governance
		///
		/// # Parameters
		/// - `state_machine`: The state machine where the oracle is deployed
		/// - `updates`: The token decimals updates
		/// - `fee`: Metadata for paying dispatch fees
		///
		/// # Errors
		/// - `OracleNotFound`: If no oracle exists for the state machine
		/// - `DispatchFailed`: If cross-chain dispatch fails
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::update_token_decimals())]
		pub fn update_token_decimals(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			updates: Vec<TokenDecimalsUpdate>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Get gateway info to access oracle address
			let gateway_info =
				Gateways::<T>::get(state_machine).ok_or(Error::<T>::GatewayNotFound)?;

			// Get oracle address from gateway params
			let oracle = gateway_info.params.price_oracle;

			// Prepare cross-chain request
			let request = RequestKind::UpdateTokenDecimals(updates.clone());
			let body = request.encode_body();

			// Dispatch cross-chain message
			Self::dispatch(state_machine, oracle, body)?;

			Self::deposit_event(Event::TokenDecimalsUpdateInitiated { state_machine, updates });

			Ok(())
		}

		/// Set the storage deposit fee for bids
		///
		/// # Parameters
		/// - `fee`: The new storage deposit fee
		#[pallet::call_index(6)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_storage_deposit_fee(origin: OriginFor<T>, fee: BalanceOf<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			StorageDepositFee::<T>::put(fee);

			Self::deposit_event(Event::StorageDepositFeeUpdated { fee });

			Ok(())
		}

		/// Submit prices for a recognized token pair across one or more amount ranges.
		///
		/// On the first submission per (account, pair), a deposit is reserved from the
		/// submitter's balance. Subsequent submissions for the same pair are free.
		/// The deposit can be withdrawn after the configured lock duration via
		/// `withdraw_price_deposit`.
		///
		/// Each entry in `entries` specifies a base token amount range and the
		/// corresponding price of the base token in terms of the quote token.
		#[pallet::call_index(7)]
		#[pallet::weight({
			T::DbWeight::get().reads(12).saturating_add(T::DbWeight::get().writes(4))
		})]
		pub fn submit_pair_price(
			origin: OriginFor<T>,
			pair_id: H256,
			entries: BoundedVec<PriceInput, T::MaxPriceEntries>,
		) -> DispatchResult {
			let submitter = ensure_signed(origin)?;

			ensure!(!entries.is_empty(), Error::<T>::EmptyPriceEntries);
			ensure!(
				entries.iter().all(|e| e.range_start <= e.range_end),
				Error::<T>::InvalidPriceRange
			);
			ensure!(RecognizedPairs::<T>::contains_key(&pair_id), Error::<T>::PairNotRecognized);

			let deposit_amount = PriceDepositAmount::<T>::get();
			ensure!(!deposit_amount.is_zero(), Error::<T>::PriceDepositsNotConfigured);

			if let Some((_, Some(_unlock_block))) = PriceDeposits::<T>::get(&submitter, &pair_id) {
				return Err(Error::<T>::WithdrawalInProgress.into());
			}

			let now = T::Dispatcher::default().timestamp().as_secs();

			// Reserve deposit on first submission per (account, pair)
			if !PriceDeposits::<T>::contains_key(&submitter, &pair_id) {
				<T as Config>::Currency::reserve(&submitter, deposit_amount)
					.map_err(|_| Error::<T>::InsufficientBalance)?;

				PriceDeposits::<T>::insert(
					&submitter,
					&pair_id,
					(deposit_amount, None::<BlockNumberFor<T>>),
				);

				Self::deposit_event(Event::PriceDepositReserved {
					submitter: submitter.clone(),
					pair_id,
					amount: deposit_amount,
				});
			}

			Self::maybe_clear_stale_prices();

			Prices::<T>::mutate(&pair_id, |stored| {
				stored.extend(entries.iter().map(|input| PriceEntry {
					range_start: input.range_start,
					range_end: input.range_end,
					price: input.price,
					timestamp: now,
				}));
			});

			Self::deposit_event(Event::PriceSubmitted { submitter, pair_id });

			Ok(())
		}

		/// Add a recognized token pair for price tracking
		#[pallet::call_index(8)]
		#[pallet::weight(T::DbWeight::get().reads(1).saturating_add(T::DbWeight::get().writes(1)))]
		pub fn add_recognized_pair(origin: OriginFor<T>, pair: TokenPair) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let pair_id = pair.pair_id();
			ensure!(!RecognizedPairs::<T>::contains_key(&pair_id), Error::<T>::PairAlreadyExists);

			RecognizedPairs::<T>::insert(&pair_id, &pair);

			Self::deposit_event(Event::RecognizedPairAdded { pair_id, pair });

			Ok(())
		}

		/// Remove a recognized token pair
		#[pallet::call_index(9)]
		#[pallet::weight(T::DbWeight::get().reads(1).saturating_add(T::DbWeight::get().writes(2)))]
		pub fn remove_recognized_pair(origin: OriginFor<T>, pair_id: H256) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(RecognizedPairs::<T>::contains_key(&pair_id), Error::<T>::PairNotRecognized);

			RecognizedPairs::<T>::remove(&pair_id);
			Prices::<T>::remove(&pair_id);

			Self::deposit_event(Event::RecognizedPairRemoved { pair_id });

			Ok(())
		}

		/// Set the price window duration
		#[pallet::call_index(10)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_price_window_duration(origin: OriginFor<T>, duration_ms: u64) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			PriceWindowDurationValue::<T>::put(duration_ms);

			Self::deposit_event(Event::PriceWindowDurationUpdated { duration_ms });

			Ok(())
		}

		/// Set the deposit amount required for price submissions
		#[pallet::call_index(11)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_price_deposit_amount(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			PriceDepositAmount::<T>::put(amount);

			Self::deposit_event(Event::PriceDepositAmountUpdated { amount });

			Ok(())
		}

		/// Set the lock duration (in blocks) for price deposits
		#[pallet::call_index(12)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_price_deposit_lock_duration(
			origin: OriginFor<T>,
			duration_blocks: BlockNumberFor<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			PriceDepositLockDuration::<T>::put(duration_blocks);

			Self::deposit_event(Event::PriceDepositLockDurationUpdated { duration_blocks });

			Ok(())
		}

		/// Withdraw a price deposit using a two-phase process.
		///
		/// **First call**: Initiates the withdrawal by recording the unlock block
		/// (current block + `PriceDepositLockDuration`). No tokens are moved.
		///
		/// **Second call** (after the unlock block has been reached): Unreserves
		/// the deposited tokens and removes the deposit record.
		///
		/// # Parameters
		/// - `pair_id`: The token pair the deposit was made for
		///
		/// # Errors
		/// - `DepositNotFound`: No deposit exists for this account and pair
		/// - `WithdrawalAlreadyInitiated`: First call was already made (waiting for unlock)
		/// - `DepositStillLocked`: The unlock block has not yet been reached
		#[pallet::call_index(13)]
		#[pallet::weight(T::DbWeight::get().reads(3).saturating_add(T::DbWeight::get().writes(1)))]
		pub fn withdraw_price_deposit(origin: OriginFor<T>, pair_id: H256) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let (deposit_amount, unlock_block) =
				PriceDeposits::<T>::get(&who, &pair_id).ok_or(Error::<T>::DepositNotFound)?;

			match unlock_block {
				None => {
					// Phase 1: Initiate withdrawal — note the unlock block
					let current_block = <frame_system::Pallet<T>>::block_number();
					let lock_duration = PriceDepositLockDuration::<T>::get();
					let unlock_at = current_block.saturating_add(lock_duration);

					PriceDeposits::<T>::insert(&who, &pair_id, (deposit_amount, Some(unlock_at)));

					Self::deposit_event(Event::PriceDepositWithdrawalInitiated {
						submitter: who,
						pair_id,
						unlock_block: unlock_at,
					});
				},
				Some(unlock_at) => {
					// Phase 2: Complete withdrawal — unreserve if unlock block reached
					let current_block = <frame_system::Pallet<T>>::block_number();
					ensure!(current_block >= unlock_at, Error::<T>::DepositStillLocked);

					<T as Config>::Currency::unreserve(&who, deposit_amount);
					PriceDeposits::<T>::remove(&who, &pair_id);

					Self::deposit_event(Event::PriceDepositWithdrawn {
						submitter: who,
						pair_id,
						amount: deposit_amount,
					});
				},
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		/// Returns the current storage deposit fee.
		/// Uses the storage value if non-zero, otherwise falls back to the Config
		/// constant.
		pub fn storage_deposit_fee() -> BalanceOf<T> {
			let fee = StorageDepositFee::<T>::get();
			if fee.is_zero() {
				T::StorageDepositFee::get()
			} else {
				fee
			}
		}

		/// Generate offchain storage key for a bid
		pub fn offchain_bid_key(commitment: &H256, filler: &T::AccountId) -> Vec<u8> {
			offchain_bid_key_raw(commitment, &filler.encode())
		}

		/// Clear all prices if this is the first submission in a new window.
		///
		/// Prices from the previous window persist until the first new submission
		/// in the new window, at which point all entries across all pairs are cleared.
		fn maybe_clear_stale_prices() {
			if !PricesClearedThisWindow::<T>::get() {
				let _ = Prices::<T>::clear(u32::MAX, None);
				PricesClearedThisWindow::<T>::put(true);
			}
		}

		/// Dispatch a cross-chain message to a gateway contract
		fn dispatch(state_machine: StateMachine, to: H160, body: Vec<u8>) -> DispatchResult {
			// Create dispatcher instance
			let dispatcher = T::Dispatcher::default();

			// Create ISMP post request
			let post = DispatchPost {
				dest: state_machine,
				from: PALLET_INTENTS_ID.to_vec(),
				to: to.0.to_vec(),
				timeout: 0, // No timeout for governance actions
				body,
			};

			let dispatch_request = DispatchRequest::Post(post);

			// Create fee metadata with zero fee (no actual fee payment for governance operations)
			let dispatcher_fee = FeeMetadata { payer: [0u8; 32].into(), fee: Zero::zero() };

			// Dispatch via ISMP
			let commitment = dispatcher
				.dispatch_request(dispatch_request, dispatcher_fee)
				.map_err(|_| Error::<T>::DispatchFailed)?;

			log::info!(
				target: "pallet-intents",
				"Dispatched cross-chain request to {:?}, commitment: {:?}",
				state_machine,
				commitment
			);

			Ok(())
		}
	}
}
