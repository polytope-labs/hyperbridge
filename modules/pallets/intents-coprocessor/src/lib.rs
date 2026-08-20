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
pub mod migrations;
#[cfg(test)]
mod tests;
pub mod types;
mod weights;

use alloc::{collections::BTreeSet, vec::Vec};
use codec::Encode as _;
use frame_support::{
	ensure,
	traits::{Currency, ReservableCurrency},
	BoundedBTreeSet, BoundedVec,
};
use ismp::{
	consensus::StateMachineId,
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	host::StateMachine,
};
use pallet_ismp::LatestStateMachineHeight;
use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use sp_core::Get;
use sp_io::offchain_index;
use sp_runtime::{
	traits::{ConstU32, Zero},
	SaturatedConversion,
};
pub use weights::WeightInfo;

use types::{
	Bid, GatewayInfo, IntentGatewayParams, PaymasterParams, PhantomOrderConfiguration,
	PhantomOrderInfo, PhantomOrderLeg, PhantomTokenPairs, RequestKind, TokenDecimalsUpdate,
	TokenInfo, MAX_PHANTOM_CHAINS, MAX_PHANTOM_ORDER_LEGS,
};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// Pallet identifier for ISMP routing
pub const PALLET_INTENTS_ID: &[u8] = b"pallet-intents";

/// Logging target for this pallet.
const LOG_TARGET: &str = "runtime::intents-coprocessor";

/// Generate the offchain storage key for a bid given raw commitment and filler bytes.
pub fn offchain_bid_key_raw(commitment: &H256, filler_encoded: &[u8]) -> Vec<u8> {
	let mut key = b"intents::bid::".to_vec();
	key.extend_from_slice(commitment.as_bytes());
	key.extend_from_slice(filler_encoded);
	key
}

/// Generate the offchain storage key for the ABI-encoded phantom order, keyed by commitment.
pub fn offchain_phantom_key(commitment: &H256) -> Vec<u8> {
	let mut key = b"intents::phantom::order::".to_vec();
	key.extend_from_slice(commitment.as_bytes());
	key
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::alloc::string::ToString;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use polkadot_sdk::sp_runtime::traits::Saturating;

	/// Current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
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

		/// How many blocks after phantom order creation bids are accepted. Fallback when
		/// the PhantomBidWindow storage value is zero.
		#[pallet::constant]
		type PhantomOrderBidWindowBlocks: Get<u32>;

		/// Origin that can perform governance actions
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

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

	/// Storage for SimplexPaymaster proxy addresses per state machine
	#[pallet::storage]
	pub type Paymasters<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachine, H160, OptionQuery>;

	/// The phantom orders active for the current interval, one bundled order per configured
	/// chain (every token pair configured for a chain rides in that chain's single order).
	/// Keeping them all here lets `place_bid` enforce the bid rules for every chain's order,
	/// not just the last one generated. Every chain generates on the same block, so the batch is
	/// replaced as a whole each cycle; removing a chain drops just its entry.
	#[pallet::storage]
	pub type CurrentPhantomOrder<T: Config> = StorageValue<
		_,
		BoundedVec<(H256, PhantomOrderInfo<BlockNumberFor<T>>), ConstU32<MAX_PHANTOM_CHAINS>>,
		OptionQuery,
	>;

	/// The block at which phantom orders were last generated, used to decide when the next
	/// interval is due. Cleared by set_phantom_order_config so generation restarts immediately.
	#[pallet::storage]
	pub type LastPhantomGeneration<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Governance-updatable bid acceptance window for phantom orders (in blocks).
	/// Falls back to PhantomOrderBidWindowBlocks when zero.
	#[pallet::storage]
	pub type PhantomBidWindow<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Governance-settable phantom order configuration, one entry per chain so a chain's token
	/// pairs are set and removed on their own without touching any other chain's. While an entry
	/// is present, the on_initialize hook emits that chain's bundled commitment every generation.
	#[pallet::storage]
	pub type PhantomOrderConfig<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineId, PhantomTokenPairs, OptionQuery>;

	/// Blocks between generations, shared by every configured chain so they all generate on the
	/// same block and their bid windows close together. Zero means generate once and never
	/// regenerate.
	#[pallet::storage]
	pub type PhantomOrderInterval<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The chains carrying a `PhantomOrderConfig` entry. Kept as a set so `on_initialize` can
	/// read the configured chains in a single storage read instead of iterating the map every
	/// block, and so the chain count stays bounded by `MAX_PHANTOM_CHAINS`.
	#[pallet::storage]
	pub type PhantomChains<T: Config> =
		StorageValue<_, BoundedBTreeSet<StateMachineId, ConstU32<MAX_PHANTOM_CHAINS>>, ValueQuery>;

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
		/// The runtime generated a new phantom order commitment. Every configured pair expands
		/// into its configured direction plus the reverse, and the legs are listed in the same
		/// order as the order's asset lists, so index `i` here is the order's leg `i`.
		PhantomOrderRegistered {
			commitment: H256,
			chain: Vec<u8>,
			created_at: BlockNumberFor<T>,
			legs: BoundedVec<PhantomOrderLeg, ConstU32<MAX_PHANTOM_ORDER_LEGS>>,
		},
		/// The phantom order bid window was updated
		PhantomBidWindowUpdated { window: u32 },
		/// The phantom order configuration was updated by governance. `chain_count` and
		/// `pair_count` cover the chains this call carried; chains it left out are unaffected.
		PhantomOrderConfigSet { chain_count: u32, pair_count: u32, interval_blocks: u32 },
		/// One chain's phantom order configuration was removed by governance. That chain stops
		/// generating and its active order, if any, no longer accepts bids.
		PhantomOrderConfigRemoved { chain: StateMachineId },
		/// A phantom order's bid window closed; the indexer can now aggregate its snapshot.
		PhantomBidWindowExhausted { commitment: H256, created_at: BlockNumberFor<T> },
		/// A gateway implementation upgrade was initiated
		GatewayUpgradeInitiated { state_machine: StateMachine, new_impl: H160 },
		/// A SimplexPaymaster deployment address was registered
		PaymasterDeploymentAdded { state_machine: StateMachine, paymaster: H160 },
		/// A paymaster implementation upgrade was initiated
		PaymasterUpgradeInitiated { state_machine: StateMachine, new_impl: H160 },
		/// Paymaster parameters update was initiated
		PaymasterParamsUpdateInitiated { state_machine: StateMachine, params: PaymasterParams },
		/// A paymaster token registration was initiated
		PaymasterTokenRegistered { state_machine: StateMachine, token: H160, oracle: H160 },
		/// A paymaster token deactivation was initiated
		PaymasterTokenDeactivated { state_machine: StateMachine, token: H160 },
		/// A paymaster asset withdrawal was initiated
		PaymasterWithdrawalInitiated { state_machine: StateMachine, token: H160, amount: U256 },
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
		/// Paymaster not found for the specified state machine
		PaymasterNotFound,
		/// Invalid user operation data
		InvalidUserOp,
		/// Failed to dispatch cross-chain request
		DispatchFailed,
		/// A bid was submitted for a phantom order after the acceptance window closed
		PhantomOrderBidWindowClosed,
		/// A filler already has a bid for this phantom order
		DuplicatePhantomBid,
		/// The effective phantom bid window is not shorter than the generation interval.
		/// They must satisfy `window < interval_blocks` so an order is never replaced on the
		/// same block its bid window closes (which would drop its exhaustion snapshot).
		PhantomBidWindowNotShorterThanInterval,
		/// The phantom order configuration carries no chains, so there is nothing to configure.
		EmptyPhantomChains,
		/// Another consensus state id is already configured for this state machine. A chain's
		/// order is identified by its state machine alone, so a second entry would generate a
		/// colliding order.
		DuplicatePhantomChain,
		/// More than `MAX_PHANTOM_CHAINS` chains would carry a configuration. Remove one first.
		TooManyPhantomChains,
		/// The chain carries no phantom order configuration to remove.
		PhantomChainNotConfigured,
		/// A configured chain carries no token pairs, so there is nothing to price on it.
		EmptyPhantomTokenPairs,
		/// Two pairs configured for one chain share the same unordered token set. Every pair
		/// expands into both directions, so the second entry would duplicate legs already in
		/// that chain's order.
		DuplicatePhantomTokenPair,
		/// A pair carries a zero standard amount. It is the denominator of every published rate,
		/// so zero is never meaningful.
		ZeroPhantomStandardAmount,
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

			// Phantom orders have stricter rules: one bid per filler, no updates, and only
			// within the configured acceptance window after the order was registered. Every
			// chain's active order is checked, not just the most recently generated one.
			if let Some(active) = CurrentPhantomOrder::<T>::get() {
				if let Some((_, info)) = active.iter().find(|(c, _)| *c == commitment) {
					let window: BlockNumberFor<T> = Self::phantom_bid_window().into();
					ensure!(
						frame_system::Pallet::<T>::block_number() <= info.created_at_block + window,
						Error::<T>::PhantomOrderBidWindowClosed
					);
					ensure!(
						!Bids::<T>::contains_key(&commitment, &filler),
						Error::<T>::DuplicatePhantomBid
					);
				}
			}

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
				let new_deployment =
					types::NewDeployment { chain: state_machine.to_string().into_bytes(), gateway };
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

		/// Set the phantom order configuration for the chains the call carries, leaving every
		/// other configured chain's token pairs untouched. The on_initialize hook generates one
		/// bundled phantom order per configured chain when the interval elapses. Also clears the
		/// active orders and the generation marker so the hook fires on the next block.
		///
		/// `config.chains` is a map, so a call may configure a single chain or several at once,
		/// and a chain absent from it keeps whatever it already had — use
		/// [`remove_phantom_order_config`](Self::remove_phantom_order_config) to stop one.
		/// `config.interval_blocks` is the exception: it is shared by every configured chain, so
		/// all of them generate on the same block and their bid windows close together, and this
		/// call sets it for all of them.
		///
		/// Each configured pair is probed in BOTH directions — the generator expands it into a
		/// forward and a reverse leg — so a pair is registered once, not once per direction.
		///
		/// ⚠ Before calling: each pair's `standard_amount` MUST be a whole number of units of its
		/// input token (`n * 10^decimals(token_a)`) and `standard_amount_b` the same multiple of
		/// `token_b`'s unit. They are the denominators of every published rate and fail silently
		/// if wrong. Keep `n` consistent across the whole config — it is the probe size, and a
		/// probe larger than a filler's per-pair cap is quoted short and publishes a wrong price.
		/// See [`PhantomTokenPair::standard_amount`](crate::types::PhantomTokenPair::standard_amount).
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_phantom_order_config(config.chains.len() as u32))]
		pub fn set_phantom_order_config(
			origin: OriginFor<T>,
			config: PhantomOrderConfiguration,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let PhantomOrderConfiguration { chains: updates, interval_blocks } = config;

			// The bid window must close strictly before the next generation so on_finalize emits
			// the exhaustions before on_initialize replaces the orders. interval_blocks == 0 means
			// generate once and never regenerate, so there is no replacement to race.
			ensure!(
				interval_blocks == 0 || Self::phantom_bid_window() < interval_blocks,
				Error::<T>::PhantomBidWindowNotShorterThanInterval
			);

			ensure!(!updates.is_empty(), Error::<T>::EmptyPhantomChains);

			for token_pairs in updates.values() {
				ensure!(!token_pairs.is_empty(), Error::<T>::EmptyPhantomTokenPairs);

				// Every pair expands into both directions, so uniqueness is on the unordered
				// token set: registering cNGN/USDC and USDC/cNGN would put the same two legs in
				// the chain's order twice and count them twice in one snapshot.
				let unordered = token_pairs
					.iter()
					.map(|pair| {
						if pair.token_a <= pair.token_b {
							(pair.token_a, pair.token_b)
						} else {
							(pair.token_b, pair.token_a)
						}
					})
					.collect::<BTreeSet<_>>();
				ensure!(
					unordered.len() == token_pairs.len(),
					Error::<T>::DuplicatePhantomTokenPair
				);

				// The amounts are the denominators of every published rate; zero can only be a
				// misconfiguration. Their actual value (one whole unit) is unknowable on-chain.
				ensure!(
					token_pairs
						.iter()
						.all(|pair| pair.standard_amount != 0 && pair.standard_amount_b != 0),
					Error::<T>::ZeroPhantomStandardAmount
				);
			}

			let mut chains = PhantomChains::<T>::get();
			for chain in updates.keys() {
				// An order is identified by its state machine alone (the commitment commits to
				// the state id, not the consensus state id), so the same state machine under a
				// second consensus state id — here or already configured — would generate a
				// colliding order.
				ensure!(
					!chains.iter().any(|id| id.state_id == chain.state_id &&
						id.consensus_state_id != chain.consensus_state_id),
					Error::<T>::DuplicatePhantomChain
				);
				chains.try_insert(*chain).map_err(|_| Error::<T>::TooManyPhantomChains)?;
			}

			let chain_count = updates.len() as u32;
			let pair_count = updates.values().map(|pairs| pairs.len() as u32).sum::<u32>();

			for (chain, token_pairs) in updates.into_iter() {
				PhantomOrderConfig::<T>::insert(chain, token_pairs);
			}
			PhantomChains::<T>::put(chains);
			PhantomOrderInterval::<T>::put(interval_blocks);
			CurrentPhantomOrder::<T>::kill();
			LastPhantomGeneration::<T>::kill();

			Self::deposit_event(Event::PhantomOrderConfigSet {
				chain_count,
				pair_count,
				interval_blocks,
			});

			Ok(())
		}

		/// Remove one chain's phantom order configuration. The chain stops generating and its
		/// active order, if any, is dropped so it no longer accepts bids. Every other chain keeps
		/// its configuration and its place in the shared generation cycle.
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::remove_phantom_order_config())]
		pub fn remove_phantom_order_config(
			origin: OriginFor<T>,
			chain: StateMachineId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(
				PhantomOrderConfig::<T>::contains_key(chain),
				Error::<T>::PhantomChainNotConfigured
			);

			PhantomOrderConfig::<T>::remove(chain);
			PhantomChains::<T>::mutate(|chains| {
				chains.remove(&chain);
			});
			// Only this chain's order is dropped; the generation marker is shared, so the other
			// chains keep the orders they are still taking bids on.
			Self::drop_active_phantom_order(&chain);

			Self::deposit_event(Event::PhantomOrderConfigRemoved { chain });

			Ok(())
		}

		/// Update the phantom order bid acceptance window.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_phantom_bid_window())]
		pub fn set_phantom_bid_window(origin: OriginFor<T>, window: u32) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Resolve the window the way phantom_bid_window() would: 0 falls back to the
			// configured constant. Enforce window < interval_blocks against the shared interval
			// so a batch is never replaced on the block its bid window closes.
			let effective_window =
				if window == 0 { T::PhantomOrderBidWindowBlocks::get() } else { window };
			let interval = PhantomOrderInterval::<T>::get();
			ensure!(
				interval == 0 || effective_window < interval,
				Error::<T>::PhantomBidWindowNotShorterThanInterval
			);

			PhantomBidWindow::<T>::put(window);

			Self::deposit_event(Event::PhantomBidWindowUpdated { window });

			Ok(())
		}

		/// Upgrade the Intent Gateway implementation behind its ERC-1967 proxy via cross-chain
		/// governance. The upgrade is authorized on the gateway by `source == hyperbridge`, the
		/// same authority used for `update_params`/`sweep_dust`.
		///
		/// # Parameters
		/// - `state_machine`: The state machine where the gateway is deployed
		/// - `new_impl`: The address of the new implementation contract
		/// - `init_data`: Optional migration calldata executed atomically against the proxy on
		///   upgrade
		///
		/// # Errors
		/// - `GatewayNotFound`: If no gateway exists for the state machine
		/// - `DispatchFailed`: If cross-chain dispatch fails
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::upgrade_gateway())]
		pub fn upgrade_gateway(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			new_impl: H160,
			init_data: Vec<u8>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let gateway_info =
				Gateways::<T>::get(state_machine).ok_or(Error::<T>::GatewayNotFound)?;

			let request = RequestKind::UpgradeContract { new_impl, init_data };
			let body = request.encode_body();

			Self::dispatch(state_machine, gateway_info.gateway, body)?;

			Self::deposit_event(Event::GatewayUpgradeInitiated { state_machine, new_impl });

			Ok(())
		}

		/// Register a SimplexPaymaster deployment address for a state machine so
		/// subsequent paymaster governance actions can target it. The contract itself
		/// is configured atomically at deploy time; this only records where it lives.
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::add_paymaster_deployment())]
		pub fn add_paymaster_deployment(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			paymaster: H160,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			Paymasters::<T>::insert(state_machine, paymaster);

			Self::deposit_event(Event::PaymasterDeploymentAdded { state_machine, paymaster });

			Ok(())
		}

		/// Upgrade the SimplexPaymaster implementation behind its ERC-1967 proxy via
		/// cross-chain governance. Authorized on the paymaster by `source == hyperbridge`.
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::upgrade_paymaster())]
		pub fn upgrade_paymaster(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			new_impl: H160,
			init_data: Vec<u8>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let paymaster =
				Paymasters::<T>::get(state_machine).ok_or(Error::<T>::PaymasterNotFound)?;

			let request = RequestKind::PaymasterUpgrade { new_impl, init_data };
			Self::dispatch(state_machine, paymaster, request.encode_body())?;

			Self::deposit_event(Event::PaymasterUpgradeInitiated { state_machine, new_impl });

			Ok(())
		}

		/// Replace the SimplexPaymaster pricing and treasury parameters wholesale.
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::update_paymaster_params())]
		pub fn update_paymaster_params(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			params: PaymasterParams,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let paymaster =
				Paymasters::<T>::get(state_machine).ok_or(Error::<T>::PaymasterNotFound)?;

			let request = RequestKind::PaymasterUpdateParams(params.clone());
			Self::dispatch(state_machine, paymaster, request.encode_body())?;

			Self::deposit_event(Event::PaymasterParamsUpdateInitiated { state_machine, params });

			Ok(())
		}

		/// Register or update a supported token and its token/USD feed on the paymaster.
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::register_paymaster_token())]
		pub fn register_paymaster_token(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			token: H160,
			oracle: H160,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let paymaster =
				Paymasters::<T>::get(state_machine).ok_or(Error::<T>::PaymasterNotFound)?;

			let request = RequestKind::PaymasterRegisterToken { token, oracle };
			Self::dispatch(state_machine, paymaster, request.encode_body())?;

			Self::deposit_event(Event::PaymasterTokenRegistered { state_machine, token, oracle });

			Ok(())
		}

		/// Deactivate a token on the paymaster (kill-switch for a misbehaving feed).
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::deactivate_paymaster_token())]
		pub fn deactivate_paymaster_token(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			token: H160,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let paymaster =
				Paymasters::<T>::get(state_machine).ok_or(Error::<T>::PaymasterNotFound)?;

			let request = RequestKind::PaymasterDeactivateToken { token };
			Self::dispatch(state_machine, paymaster, request.encode_body())?;

			Self::deposit_event(Event::PaymasterTokenDeactivated { state_machine, token });

			Ok(())
		}

		/// Sweep paymaster assets to its treasury: an ERC-20 surplus balance, or the
		/// EntryPoint deposit when `token` is the zero address.
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::withdraw_paymaster_assets())]
		pub fn withdraw_paymaster_assets(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			token: H160,
			amount: U256,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let paymaster =
				Paymasters::<T>::get(state_machine).ok_or(Error::<T>::PaymasterNotFound)?;

			let request = RequestKind::PaymasterWithdrawAssets { token, amount };
			Self::dispatch(state_machine, paymaster, request.encode_body())?;

			Self::deposit_event(Event::PaymasterWithdrawalInitiated {
				state_machine,
				token,
				amount,
			});

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// The configured chains live in a set precisely so this read is a single one; the
			// config map is only touched on a generation block, once per configured chain.
			let chains = PhantomChains::<T>::get();
			if chains.is_empty() {
				// Reserve the read on_finalize performs on CurrentPhantomOrder.
				return T::DbWeight::get().reads(2);
			}

			let interval_blocks = PhantomOrderInterval::<T>::get();
			let should_generate = match LastPhantomGeneration::<T>::get() {
				None => true,
				Some(last) => {
					let interval: BlockNumberFor<T> = interval_blocks.into();
					!interval.is_zero() && n >= last.saturating_add(interval)
				},
			};

			// reads here (chains + interval + last_generation) plus the reads on_finalize
			// performs.
			if !should_generate {
				return T::DbWeight::get().reads(5);
			}

			// One bundled order per configured chain, each carrying both directions of every
			// pair configured for it. The weight sums the per-chain benchmark (encoding and
			// hashing the legs is what its linear term pays for) plus each chain's config and
			// confirmed height reads; the two extra reads are the ones on_finalize performs on
			// the batch.
			let mut weight = T::DbWeight::get()
				.reads(5)
				.saturating_add(T::DbWeight::get().reads(2 * chains.len() as u64));
			let mut batch: BoundedVec<
				(H256, PhantomOrderInfo<BlockNumberFor<T>>),
				ConstU32<MAX_PHANTOM_CHAINS>,
			> = BoundedVec::new();

			for chain in chains.iter() {
				let Some(token_pairs) = PhantomOrderConfig::<T>::get(chain) else {
					continue;
				};

				// Phantom orders carry the latest confirmed height as their deadline so they
				// read as already expired on-chain and can never be executed for real. A chain
				// with no confirmed height yet is skipped.
				let Some(deadline) = LatestStateMachineHeight::<T>::get(*chain) else {
					log::warn!(
						target: LOG_TARGET,
						"No confirmed state machine height for {:?}, skipping phantom order generation",
						chain,
					);
					continue;
				};

				let chain_bytes = chain.state_id.to_string().into_bytes();
				let legs = types::phantom_order_legs(&token_pairs);
				let (commitment, order_bytes) = types::phantom_order_commitment(
					n.saturated_into::<u64>(),
					&chain_bytes,
					&legs,
					deadline,
				);
				offchain_index::set(&offchain_phantom_key(&commitment), &order_bytes);

				let info = PhantomOrderInfo { created_at_block: n, chain: chain_bytes.clone() };
				let _ = batch.try_push((commitment, info));
				Self::deposit_event(Event::PhantomOrderRegistered {
					commitment,
					chain: chain_bytes,
					created_at: n,
					legs: BoundedVec::truncate_from(legs),
				});
				weight = weight.saturating_add(T::WeightInfo::generate_phantom_order(
					token_pairs.len() as u32,
				));
			}

			// No chain had a confirmed height: leave the previous batch and the generation
			// marker untouched so the hook retries next block instead of waiting an interval.
			if batch.is_empty() {
				return weight;
			}

			CurrentPhantomOrder::<T>::put(batch);
			LastPhantomGeneration::<T>::put(n);

			weight
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			// Signal each active commitment on the block its bid window closes so the indexer can
			// aggregate that order's snapshot. Emitted in on_finalize (after all extrinsics) so any
			// bid placed in the window-closing block is already in storage when the snapshot is
			// taken. The bid window is expected to be shorter than the generation interval, so the
			// active batch is never replaced by on_initialize on the same block its window closes.
			let Some(active) = CurrentPhantomOrder::<T>::get() else {
				return;
			};
			let window: BlockNumberFor<T> = Self::phantom_bid_window().into();
			for (commitment, info) in active.iter() {
				if n == info.created_at_block.saturating_add(window) {
					Self::deposit_event(Event::PhantomBidWindowExhausted {
						commitment: *commitment,
						created_at: info.created_at_block,
					});
				}
			}
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

		/// Drop a chain's entry from the active phantom batch, leaving the other chains' orders
		/// (and their bid windows) alone. Called when a chain is reconfigured or removed, so no
		/// bid can be placed against a commitment its configuration no longer describes.
		fn drop_active_phantom_order(chain: &StateMachineId) {
			let chain_bytes = chain.state_id.to_string().into_bytes();
			CurrentPhantomOrder::<T>::mutate(|active| {
				let Some(batch) = active else {
					return;
				};
				batch.retain(|(_, info)| info.chain != chain_bytes);
				if batch.is_empty() {
					*active = None;
				}
			});
		}

		pub fn phantom_bid_window() -> u32 {
			let window = PhantomBidWindow::<T>::get();
			if window == 0 {
				T::PhantomOrderBidWindowBlocks::get()
			} else {
				window
			}
		}

		/// Generate offchain storage key for a bid
		pub fn offchain_bid_key(commitment: &H256, filler: &T::AccountId) -> Vec<u8> {
			offchain_bid_key_raw(commitment, &filler.encode())
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
				target: LOG_TARGET,
				"Dispatched cross-chain request to {:?}, commitment: {:?}",
				state_machine,
				commitment
			);

			Ok(())
		}
	}
}
