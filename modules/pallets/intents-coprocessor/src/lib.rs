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
	messaging::Proof,
};
use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use sp_core::Get;
use sp_io::offchain_index;
use sp_runtime::traits::{ConstU32, Zero};
pub use weights::WeightInfo;

use types::{
	Bid, GatewayInfo, IntentGatewayParams, PriceEntry, RequestKind, TokenDecimalsUpdate, TokenInfo,
	TokenPair,
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
	use alloc::vec;
	use crate::alloc::string::ToString;
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

		/// The treasury account that receives unverified submission fees
		type TreasuryAccount: Get<Self::AccountId>;

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

	/// Commitments that have already been used for price submissions
	#[pallet::storage]
	pub type UsedCommitments<T: Config> = StorageMap<_, Blake2_128Concat, H256, bool, ValueQuery>;

	/// Start timestamp (in seconds) of the current price window
	#[pallet::storage]
	pub type PriceWindowStart<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Price window duration in milliseconds
	#[pallet::storage]
	pub type PriceWindowDurationValue<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Proof freshness threshold in seconds
	#[pallet::storage]
	pub type ProofFreshnessThresholdValue<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Verified (high confidence) price entries per pair, from fillers with proofs
	#[pallet::storage]
	pub type VerifiedPrices<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, Vec<PriceEntry<T::AccountId>>, ValueQuery>;

	/// Unverified (low confidence) price entries per pair, from anyone without proofs
	#[pallet::storage]
	pub type UnverifiedPrices<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, Vec<PriceEntry<T::AccountId>>, ValueQuery>;

	/// Nonces for EVM addresses to prevent signature replay.
	/// Keyed by the 20-byte EVM address.
	#[pallet::storage]
	pub type EvmNonces<T: Config> =
		StorageMap<_, Blake2_128Concat, H160, u64, ValueQuery>;

	/// Maximum number of unverified price submissions per pair
	#[pallet::storage]
	pub type MaxUnverifiedSubmissions<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Fee charged for unverified price submissions (in native/bridge tokens)
	#[pallet::storage]
	pub type UnverifiedSubmissionFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
				// New window: clear UsedCommitments (safe because freshness threshold
				// prevents old proofs from being replayed) and update the window start.
				// Price entries (VerifiedPrices, UnverifiedPrices) are NOT cleared here —
				// they persist so yesterday's prices remain readable. They are cleared
				// lazily on the first submission per pair in the new window.
				let used_result = UsedCommitments::<T>::clear(u32::MAX, None);
				PriceWindowStart::<T>::put(now);
				PricesClearedThisWindow::<T>::put(false);

				let cleared = used_result.unique.saturating_add(1);
				T::DbWeight::get()
					.reads(3)
					.saturating_add(T::DbWeight::get().writes(cleared.into()))
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
		/// A verified price was submitted (high confidence)
		PriceSubmitted { filler: T::AccountId, pair_id: H256, price: U256 },
		/// Price window duration was updated
		PriceWindowDurationUpdated { duration_ms: u64 },
		/// Proof freshness threshold was updated
		ProofFreshnessThresholdUpdated { threshold_secs: u64 },
		/// An unverified price was submitted (low confidence)
		UnverifiedPriceSubmitted { submitter: T::AccountId, pair_id: H256, price: U256 },
		/// Max unverified submissions per pair was updated
		MaxUnverifiedSubmissionsUpdated { max: u32 },
		/// Unverified submission fee was updated
		UnverifiedSubmissionFeeUpdated { fee: BalanceOf<T> },
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
		/// Non-membership proof verification failed
		NonMembershipProofFailed,
		/// Membership proof verification failed
		MembershipProofFailed,
		/// The time gap between the two proofs exceeds the freshness threshold
		ProofNotFresh,
		/// State proof verification failed
		ProofVerificationFailed,
		/// Token pair already exists
		PairAlreadyExists,
		/// This commitment has already been used for a price submission
		CommitmentAlreadyUsed,
		/// EVM signature verification failed
		InvalidSignature,
		/// The recovered EVM address does not match the filler from the proof
		SignerMismatch,
		/// Unverified submissions are not configured (max or fee is zero)
		UnverifiedSubmissionsNotConfigured,
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

		/// Submit a price for a recognized token pair.
		///
		/// This extrinsic supports two submission modes. When `verification` is provided,
		/// the submission is treated as high confidence: the filler supplies ISMP state
		/// proofs and an EVM signature to prove they filled the order and own the
		/// submitting account (bound by an on-chain nonce). When `verification` is
		/// `None`, anyone may submit a low confidence price by paying bridge tokens.
		/// Unverified entries are capped per pair with FIFO replacement.
		///
		/// The `pair_id` identifies the token pair, and `price` is the price of the base
		/// token in terms of the quote token.
		#[pallet::call_index(7)]
		#[pallet::weight({
			T::DbWeight::get().reads(12).saturating_add(T::DbWeight::get().writes(4))
		})]
		pub fn submit_pair_price(
			origin: OriginFor<T>,
			pair_id: H256,
			price: U256,
			verification: Option<types::PriceVerificationData>,
		) -> DispatchResult {
			let submitter = ensure_signed(origin)?;

			ensure!(RecognizedPairs::<T>::contains_key(&pair_id), Error::<T>::PairNotRecognized);

			let now = T::Dispatcher::default().timestamp().as_secs();

			if let Some(v) = verification {
				Self::submit_verified_price(submitter, pair_id, price, v, now)?;
			} else {
				Self::submit_unverified_price(submitter, pair_id, price, now)?;
			}

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
		#[pallet::weight(T::DbWeight::get().reads(1).saturating_add(T::DbWeight::get().writes(3)))]
		pub fn remove_recognized_pair(origin: OriginFor<T>, pair_id: H256) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(RecognizedPairs::<T>::contains_key(&pair_id), Error::<T>::PairNotRecognized);

			RecognizedPairs::<T>::remove(&pair_id);
			VerifiedPrices::<T>::remove(&pair_id);
			UnverifiedPrices::<T>::remove(&pair_id);

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

		/// Set the proof freshness threshold
		#[pallet::call_index(11)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_proof_freshness_threshold(
			origin: OriginFor<T>,
			threshold_secs: u64,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ProofFreshnessThresholdValue::<T>::put(threshold_secs);

			Self::deposit_event(Event::ProofFreshnessThresholdUpdated { threshold_secs });

			Ok(())
		}

		/// Set the maximum number of unverified price submissions per pair
		#[pallet::call_index(12)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_max_unverified_submissions(
			origin: OriginFor<T>,
			max: u32,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			MaxUnverifiedSubmissions::<T>::put(max);

			Self::deposit_event(Event::MaxUnverifiedSubmissionsUpdated { max });

			Ok(())
		}

		/// Set the fee charged for unverified price submissions
		#[pallet::call_index(13)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_unverified_submission_fee(
			origin: OriginFor<T>,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			UnverifiedSubmissionFee::<T>::put(fee);

			Self::deposit_event(Event::UnverifiedSubmissionFeeUpdated { fee });

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

		/// Process a verified (high confidence) price submission with ISMP proofs and
		/// EVM signature verification.
		fn submit_verified_price(
			submitter: T::AccountId,
			pair_id: H256,
			price: U256,
			v: types::PriceVerificationData,
			now: u64,
		) -> DispatchResult {
			ensure!(
				!UsedCommitments::<T>::get(&v.commitment),
				Error::<T>::CommitmentAlreadyUsed
			);

			let gateway_info =
				Gateways::<T>::get(v.state_machine).ok_or(Error::<T>::GatewayNotFound)?;

			let filler_address =
				Self::verify_fill_proofs(&v, &gateway_info, now)?;

			Self::verify_evm_signature(&v, &pair_id, &price, filler_address)?;

			UsedCommitments::<T>::insert(&v.commitment, true);

			Self::maybe_clear_stale_prices();

			VerifiedPrices::<T>::mutate(&pair_id, |entries| {
				entries.push(PriceEntry {
					submitter: submitter.clone(),
					price,
					timestamp: now,
				});
			});

			Self::deposit_event(Event::PriceSubmitted { filler: submitter, pair_id, price });

			Ok(())
		}

		/// Process an unverified (low confidence) price submission. Charges a fee
		/// and stores with FIFO replacement if the cap is reached.
		fn submit_unverified_price(
			submitter: T::AccountId,
			pair_id: H256,
			price: U256,
			now: u64,
		) -> DispatchResult {
			let max = MaxUnverifiedSubmissions::<T>::get();
			let fee = UnverifiedSubmissionFee::<T>::get();
			ensure!(
				max > 0 && !fee.is_zero(),
				Error::<T>::UnverifiedSubmissionsNotConfigured
			);

			<T as Config>::Currency::transfer(
				&submitter,
				&T::TreasuryAccount::get(),
				fee,
				frame_support::traits::ExistenceRequirement::KeepAlive,
			)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

			Self::maybe_clear_stale_prices();

			UnverifiedPrices::<T>::mutate(&pair_id, |entries| {
				if entries.len() >= max as usize {
					entries.remove(0);
				}
				entries.push(PriceEntry {
					submitter: submitter.clone(),
					price,
					timestamp: now,
				});
			});

			Self::deposit_event(Event::UnverifiedPriceSubmitted { submitter, pair_id, price });

			Ok(())
		}

		/// Verify ISMP state proofs for a fill order and return the filler's EVM address.
		/// Confirms non-membership at H1, membership at H2, and that both proofs fall
		/// within the freshness threshold.
		fn verify_fill_proofs(
			v: &types::PriceVerificationData,
			gateway_info: &GatewayInfo,
			now: u64,
		) -> Result<H160, DispatchError> {
			let host = T::Dispatcher::default();

			// 52-byte key: gateway address (20) + storage slot (32)
			let storage_key =
				types::filled_storage_key(&gateway_info.gateway, &v.commitment);

			// Get state commitments for both proof heights
			let commitment_h1 = host
				.state_machine_commitment(v.non_membership_proof.height)
				.map_err(|_| Error::<T>::ProofVerificationFailed)?;
			let commitment_h2 = host
				.state_machine_commitment(v.membership_proof.height)
				.map_err(|_| Error::<T>::ProofVerificationFailed)?;

			// Validate state machine clients
			let state_machine_client_h1 =
				ismp::handlers::validate_state_machine(&host, v.non_membership_proof.height)
					.map_err(|_| Error::<T>::ProofVerificationFailed)?;
			let state_machine_client_h2 =
				ismp::handlers::validate_state_machine(&host, v.membership_proof.height)
					.map_err(|_| Error::<T>::ProofVerificationFailed)?;

			// Verify non-membership proof: order was not filled at H1
			let non_membership_result = state_machine_client_h1
				.verify_state_proof(
					&host,
					vec![storage_key.clone()],
					commitment_h1,
					&v.non_membership_proof,
				)
				.map_err(|_| Error::<T>::NonMembershipProofFailed)?;

			let value_at_h1 = non_membership_result
				.get(&storage_key)
				.ok_or(Error::<T>::NonMembershipProofFailed)?;
			ensure!(value_at_h1.is_none(), Error::<T>::NonMembershipProofFailed);

			// Verify membership proof: order was filled at H2
			let membership_result = state_machine_client_h2
				.verify_state_proof(
					&host,
					vec![storage_key.clone()],
					commitment_h2,
					&v.membership_proof,
				)
				.map_err(|_| Error::<T>::MembershipProofFailed)?;

			let filler_bytes = membership_result
				.get(&storage_key)
				.ok_or(Error::<T>::MembershipProofFailed)?
				.as_ref()
				.ok_or(Error::<T>::MembershipProofFailed)?;

			// Extract the filler address from the proof value
			// EVM addresses are 20 bytes, left-padded to 32 bytes in storage
			let filler_address = H160::from_slice(
				filler_bytes.get(12..32).ok_or(Error::<T>::MembershipProofFailed)?,
			);
			ensure!(filler_address != H160::zero(), Error::<T>::MembershipProofFailed);

			// Check proof freshness
			let threshold = ProofFreshnessThresholdValue::<T>::get();
			let proof_gap =
				commitment_h2.timestamp.saturating_sub(commitment_h1.timestamp);
			ensure!(proof_gap <= threshold, Error::<T>::ProofNotFresh);
			let age = now.saturating_sub(commitment_h2.timestamp);
			ensure!(age <= threshold, Error::<T>::ProofNotFresh);

			Ok(filler_address)
		}

		/// Verify the EVM signature proving the filler owns the submitting account.
		/// Recovers the signer from the signature over `(nonce, pair_id, price)`,
		/// checks that it matches the filler from the proof, and increments the nonce.
		fn verify_evm_signature(
			v: &types::PriceVerificationData,
			pair_id: &H256,
			price: &U256,
			filler_address: H160,
		) -> DispatchResult {
			let evm_address_bytes = match &v.evm_signature {
				crypto_utils::verification::Signature::Evm { address, .. } =>
					address.clone(),
				_ => return Err(Error::<T>::InvalidSignature.into()),
			};

			ensure!(evm_address_bytes.len() == 20, Error::<T>::InvalidSignature);
			let evm_address = H160::from_slice(&evm_address_bytes);

			let nonce = EvmNonces::<T>::get(evm_address);
			let msg = types::price_signature_message(nonce, pair_id, price);

			let recovered = v
				.evm_signature
				.verify(&msg, None)
				.map_err(|_| Error::<T>::InvalidSignature)?;
			ensure!(recovered == evm_address_bytes, Error::<T>::SignerMismatch);

			ensure!(
				recovered.len() == 20 && H160::from_slice(&recovered) == filler_address,
				Error::<T>::SignerMismatch
			);

			EvmNonces::<T>::insert(evm_address, nonce.saturating_add(1));

			Ok(())
		}

		/// Clear all prices if this is the first submission in a new window.
		///
		/// Prices from the previous window persist until the first new submission
		/// in the new window, at which point all verified and unverified entries
		/// across all pairs are cleared.
		fn maybe_clear_stale_prices() {
			if !PricesClearedThisWindow::<T>::get() {
				let _ = VerifiedPrices::<T>::clear(u32::MAX, None);
				let _ = UnverifiedPrices::<T>::clear(u32::MAX, None);
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
