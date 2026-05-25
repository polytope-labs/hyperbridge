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

//! Enables collators keep hyperbridge safe by vetoing fraudulent state
//! commitments and by blacklisting fraudulent rollup claims (opstack dispute
//! games / arbitrum assertions) before they reach the consensus verifier.
//!
//! The set of accounts allowed to act is the active collator set, sourced from
//! the runtime's `IsCollator` predicate. Every action requires two distinct
//! collators to finalize.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub use extension::PrioritizeVeto;
pub use pallet::*;
use polkadot_sdk::*;

mod extension;

use ismp::consensus::StateMachineId;
use primitive_types::{H160, H256};

/// Read-only view of the blacklists, consumed by the optimism / arbitrum
/// consensus verifier pallets to gate `verify_consensus`. Implemented for
/// [`Pallet`] by reading [`BlacklistedDisputeGames`] and
/// [`BlacklistedArbitrumClaims`].
pub trait FishermanBlacklist {
	/// True iff a (state_machine, opstack dispute-game proxy) pair has been blacklisted.
	fn is_dispute_game_blacklisted(state_machine_id: StateMachineId, proxy: H160) -> bool;
	/// True iff a (state_machine, arbitrum claim hash) pair has been blacklisted. The claim
	/// hash is the BoLD `assertionHash` for BoLD updates or
	/// `keccak256(state_hash || node_num.to_be_bytes())` for Orbit/AnyTrust updates.
	fn is_arbitrum_claim_blacklisted(state_machine_id: StateMachineId, claim: H256) -> bool;
}

/// No-op blacklist for runtimes that don't wire `pallet-fishermen`.
impl FishermanBlacklist for () {
	fn is_dispute_game_blacklisted(_: StateMachineId, _: H160) -> bool {
		false
	}
	fn is_arbitrum_claim_blacklisted(_: StateMachineId, _: H256) -> bool {
		false
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::Pays, pallet_prelude::*, traits::Contains};
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::{StateCommitment, StateMachineHeight, StateMachineId},
		events::StateCommitmentVetoed,
		host::IsmpHost,
	};
	use primitive_types::{H160, H256};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;

		/// Predicate that returns true when an account is currently entitled to
		/// submit a veto or blacklist call. Runtimes wire this to the active
		/// collator set (e.g. session validators intersected with
		/// collator-manager controllers).
		type IsCollator: Contains<Self::AccountId>;
	}

	/// Heights with a single recorded veto, awaiting a second distinct collator
	/// to finalize the veto.
	#[pallet::storage]
	#[pallet::getter(fn pending_vetoes)]
	pub type PendingVetoes<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineHeight, T::AccountId, OptionQuery>;

	/// Half-quorum half of [`BlacklistedDisputeGames`]: the first collator to
	/// flag a (state_machine, proxy) pair is recorded here; a distinct second
	/// collator finalizes the blacklist.
	#[pallet::storage]
	#[pallet::getter(fn pending_dispute_game_blacklist)]
	pub type PendingDisputeGameBlacklist<T: Config> =
		StorageMap<_, Blake2_128Concat, (StateMachineId, H160), T::AccountId, OptionQuery>;

	/// Finalized opstack dispute-game blacklist. Keyed by `(state_machine_id,
	/// dispute_game_proxy)`; the value is `(first_collator, second_collator)` — the two
	/// distinct fishermen whose calls quorum-finalized the entry. Read by
	/// `pallet-ismp-optimism::verify_consensus` to refuse `OpFaultProofGames` proofs that
	/// reference a blacklisted proxy; the recorded fishermen are kept for auditability and
	/// reputation accounting downstream.
	#[pallet::storage]
	#[pallet::getter(fn blacklisted_dispute_games)]
	pub type BlacklistedDisputeGames<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		Blake2_128Concat,
		H160,
		(T::AccountId, T::AccountId),
		OptionQuery,
	>;

	/// Half-quorum half of [`BlacklistedArbitrumClaims`].
	#[pallet::storage]
	#[pallet::getter(fn pending_arbitrum_claim_blacklist)]
	pub type PendingArbitrumClaimBlacklist<T: Config> =
		StorageMap<_, Blake2_128Concat, (StateMachineId, H256), T::AccountId, OptionQuery>;

	/// Finalized arbitrum claim blacklist. Keyed by `(state_machine_id, claim_hash)` where
	/// `claim_hash` is the BoLD `assertionHash` for BoLD updates or
	/// `keccak256(state_hash || node_num.to_be_bytes())` for Orbit/AnyTrust updates; the
	/// value is `(first_collator, second_collator)` — the two distinct fishermen whose calls
	/// quorum-finalized the entry. Read by `pallet-ismp-arbitrum::verify_consensus`; the
	/// recorded fishermen are kept for auditability and reputation accounting downstream.
	#[pallet::storage]
	#[pallet::getter(fn blacklisted_arbitrum_claims)]
	pub type BlacklistedArbitrumClaims<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		Blake2_128Concat,
		H256,
		(T::AccountId, T::AccountId),
		OptionQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Caller is not in the active collator set.
		UnauthorizedAction,
		/// State commitment was not found.
		VetoFailed,
		/// Invalid veto request (e.g. same collator submitted twice).
		InvalidVeto,
		/// Invalid blacklist request (e.g. same collator submitted twice).
		InvalidBlacklist,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The provided state commitment was vetoed at `height`.
		StateCommitmentVetoed { height: StateMachineHeight, commitment: StateCommitment },
		/// A first veto for `height` has been noted, awaiting a second distinct
		/// collator to finalize.
		VetoNoted { height: StateMachineHeight, fisherman: T::AccountId },
		/// A first blacklist for the given opstack dispute-game proxy has been
		/// noted, awaiting a second distinct collator to finalize.
		DisputeGameBlacklistNoted {
			state_machine_id: StateMachineId,
			proxy: H160,
			fisherman: T::AccountId,
		},
		/// The given opstack dispute-game proxy has been blacklisted by the two listed
		/// collators (the first one to submit and the second distinct one who finalized).
		DisputeGameBlacklisted {
			state_machine_id: StateMachineId,
			proxy: H160,
			first_fisherman: T::AccountId,
			second_fisherman: T::AccountId,
		},
		/// A first blacklist for the given arbitrum claim hash has been noted,
		/// awaiting a second distinct collator to finalize.
		ArbitrumClaimBlacklistNoted {
			state_machine_id: StateMachineId,
			claim: H256,
			fisherman: T::AccountId,
		},
		/// The given arbitrum claim hash has been blacklisted by the two listed collators (the
		/// first one to submit and the second distinct one who finalized).
		ArbitrumClaimBlacklisted {
			state_machine_id: StateMachineId,
			claim: H256,
			first_fisherman: T::AccountId,
			second_fisherman: T::AccountId,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
	{
		/// A collator has determined that some [`StateCommitment`] (which is ideally still in its
		/// challenge period) is in fact fraudulent and misrepresentative of the state changes at
		/// the provided height. This allows them to veto the state commitment. They aren't
		/// required to provide any proofs for this. Successful veto requires two distinct
		/// collators.
		///
		/// Dispatches with `Pays::No`. The on-chain `IsCollator` check is the DOS guard, so the
		/// signer does not need to hold a balance.
		#[pallet::call_index(0)]
		#[pallet::weight((<T as frame_system::Config>::DbWeight::get().reads_writes(2, 3), Pays::No))]
		pub fn veto_state_commitment(
			origin: OriginFor<T>,
			height: StateMachineHeight,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			ensure!(T::IsCollator::contains(&account), Error::<T>::UnauthorizedAction);

			if let Some(prev_veto) = PendingVetoes::<T>::get(height) {
				if account == prev_veto {
					Err(Error::<T>::InvalidVeto)?
				}
				let ismp_host = <T as Config>::IsmpHost::default();
				let commitment = ismp_host
					.state_machine_commitment(height)
					.map_err(|_| Error::<T>::VetoFailed)?;
				ismp_host.delete_state_commitment(height).map_err(|_| Error::<T>::VetoFailed)?;
				PendingVetoes::<T>::remove(height);

				Self::deposit_event(Event::StateCommitmentVetoed { height, commitment });
				pallet_ismp::Pallet::<T>::deposit_event(
					ismp::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
						height,
						fisherman: account.as_ref().to_vec(),
					})
					.into(),
				);
			} else {
				PendingVetoes::<T>::insert(height, account.clone());
				Self::deposit_event(Event::VetoNoted { height, fisherman: account });
			}

			Ok(())
		}

		/// A collator has determined that the opstack dispute game at `proxy` (registered against
		/// the configured `DisputeGameFactory` for `state_machine_id`) is fraudulent — typically
		/// because the off-chain fisherman watcher verified the claimed L2 output root against a
		/// supermajority (2/3·N + 1) of L2 RPC endpoints and observed a mismatch (or a quorum of
		/// the L2 height being absent).
		///
		/// Successful blacklist requires two distinct collators. Once finalized, the entry is
		/// permanent (there is no `unblacklist` extrinsic) — the consensus verifier will refuse
		/// any future `OpFaultProofGames` consensus proof that references this proxy.
		///
		/// Dispatches with `Pays::No`. The on-chain `IsCollator` check is the DOS guard.
		#[pallet::call_index(1)]
		#[pallet::weight((<T as frame_system::Config>::DbWeight::get().reads_writes(2, 2), Pays::No))]
		pub fn blacklist_dispute_game(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			proxy: H160,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			ensure!(T::IsCollator::contains(&account), Error::<T>::UnauthorizedAction);

			// Already finalized: silently succeed so a slow second collator doesn't fail loudly.
			if BlacklistedDisputeGames::<T>::contains_key(state_machine_id, proxy) {
				return Ok(());
			}

			let key = (state_machine_id, proxy);
			if let Some(prev) = PendingDisputeGameBlacklist::<T>::get(&key) {
				if account == prev {
					Err(Error::<T>::InvalidBlacklist)?
				}
				PendingDisputeGameBlacklist::<T>::remove(&key);
				BlacklistedDisputeGames::<T>::insert(
					state_machine_id,
					proxy,
					(prev.clone(), account.clone()),
				);
				Self::deposit_event(Event::DisputeGameBlacklisted {
					state_machine_id,
					proxy,
					first_fisherman: prev,
					second_fisherman: account,
				});
			} else {
				PendingDisputeGameBlacklist::<T>::insert(key, account.clone());
				Self::deposit_event(Event::DisputeGameBlacklistNoted {
					state_machine_id,
					proxy,
					fisherman: account,
				});
			}

			Ok(())
		}

		/// A collator has determined that the arbitrum claim identified by `claim` (a BoLD
		/// `assertionHash` or, for Orbit/AnyTrust, a derived hash of `state_hash || node_num`) is
		/// fraudulent.
		///
		/// Successful blacklist requires two distinct collators. Once finalized, the entry is
		/// permanent — the consensus verifier will refuse any future `ArbitrumOrbit` or
		/// `ArbitrumBold` consensus proof that resolves to this claim hash.
		///
		/// Dispatches with `Pays::No`. The on-chain `IsCollator` check is the DOS guard.
		#[pallet::call_index(2)]
		#[pallet::weight((<T as frame_system::Config>::DbWeight::get().reads_writes(2, 2), Pays::No))]
		pub fn blacklist_arbitrum_claim(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			claim: H256,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			ensure!(T::IsCollator::contains(&account), Error::<T>::UnauthorizedAction);

			if BlacklistedArbitrumClaims::<T>::contains_key(state_machine_id, claim) {
				return Ok(());
			}

			let key = (state_machine_id, claim);
			if let Some(prev) = PendingArbitrumClaimBlacklist::<T>::get(&key) {
				if account == prev {
					Err(Error::<T>::InvalidBlacklist)?
				}
				PendingArbitrumClaimBlacklist::<T>::remove(&key);
				BlacklistedArbitrumClaims::<T>::insert(
					state_machine_id,
					claim,
					(prev.clone(), account.clone()),
				);
				Self::deposit_event(Event::ArbitrumClaimBlacklisted {
					state_machine_id,
					claim,
					first_fisherman: prev,
					second_fisherman: account,
				});
			} else {
				PendingArbitrumClaimBlacklist::<T>::insert(key, account.clone());
				Self::deposit_event(Event::ArbitrumClaimBlacklistNoted {
					state_machine_id,
					claim,
					fisherman: account,
				});
			}

			Ok(())
		}
	}
}

impl<T: Config> FishermanBlacklist for Pallet<T> {
	fn is_dispute_game_blacklisted(state_machine_id: StateMachineId, proxy: H160) -> bool {
		BlacklistedDisputeGames::<T>::contains_key(state_machine_id, proxy)
	}

	fn is_arbitrum_claim_blacklisted(state_machine_id: StateMachineId, claim: H256) -> bool {
		BlacklistedArbitrumClaims::<T>::contains_key(state_machine_id, claim)
	}
}
