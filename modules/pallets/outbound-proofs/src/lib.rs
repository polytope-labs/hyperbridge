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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod types;
pub mod verifier;

mod benchmarking;

use polkadot_sdk::*;

pub use pallet::*;
pub use types::{BeefyAuthoritySet, BeefyConsensusState, EpochInfo, PendingProofInfo, ProofMetadata};

pub trait ProofVerifier {
	fn verify(
		trusted_state: &BeefyConsensusState,
		proof: &[u8],
	) -> Result<BeefyConsensusState, frame_support::pallet_prelude::DispatchError>;
}

/// Production SP1 BEEFY proof verifier.
/// Reads the verification key hash from pallet storage and performs
/// full verification mirroring SP1Beefy.sol.
#[cfg(feature = "sp1")]
pub struct Sp1ProofVerifier<T>(core::marker::PhantomData<T>);

#[cfg(feature = "sp1")]
impl<T: pallet::Config> ProofVerifier for Sp1ProofVerifier<T> {
	fn verify(
		trusted_state: &BeefyConsensusState,
		proof: &[u8],
	) -> Result<BeefyConsensusState, frame_support::pallet_prelude::DispatchError> {
		let vkey_bytes = pallet::Sp1VkeyHash::<T>::get();
		let vkey_hash = core::str::from_utf8(&vkey_bytes)
			.map_err(|_| frame_support::pallet_prelude::DispatchError::Other("invalid vkey hash encoding"))?;

		let result = verifier::verify_beefy_proof(trusted_state, proof, vkey_hash)
			.map_err(|e| frame_support::pallet_prelude::DispatchError::Other(match e {
				verifier::VerificationError::DecodeFailed => "proof decode failed",
				verifier::VerificationError::StaleHeight => "stale proof height",
				verifier::VerificationError::UnknownAuthoritySet => "unknown authority set",
				verifier::VerificationError::InvalidProof => "SP1 proof verification failed",
			}))?;

		Ok(result.new_state)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::Preservation,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type ProofVerifier: ProofVerifier;

		type Currency: Mutate<Self::AccountId>;

		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		#[pallet::constant]
		type MaxProofSize: Get<u32>;

		#[pallet::constant]
		type MaxStoredProofs: Get<u32>;

		type WeightInfo: WeightInfo;
	}

	pub trait WeightInfo {
		fn submit_proof() -> Weight;
		fn set_proof_reward() -> Weight;
		fn set_sp1_vkey_hash() -> Weight;
	}

	#[pallet::storage]
	pub type UnprovenEpochs<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, EpochInfo, OptionQuery>;

	#[pallet::storage]
	pub type UnprovenHeights<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, PendingProofInfo<BlockNumberFor<T>>, OptionQuery>;

	#[pallet::storage]
	pub type ProvenHeights<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u64,
		ProofMetadata<T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type RecentProofs<T: Config> = StorageValue<
		_,
		BoundedVec<ProofMetadata<T::AccountId, BlockNumberFor<T>>, T::MaxStoredProofs>,
		ValueQuery,
	>;

	/// The last parachain block number where an ISMP message was dispatched
	#[pallet::storage]
	pub type LatestMessageBlock<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// The last proven parachain block height
	#[pallet::storage]
	pub type LatestProvenParachainHeight<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Current BEEFY authority set epoch
	#[pallet::storage]
	pub type CurrentEpoch<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// BEEFY consensus state — tracks authority sets and latest proven relay height
	#[pallet::storage]
	pub type ConsensusState<T: Config> = StorageValue<_, BeefyConsensusState, ValueQuery>;

	/// Reward amount per valid proof — updatable via governance
	#[pallet::storage]
	pub type ProofReward<T: Config> =
		StorageValue<_, <T::Currency as Inspect<T::AccountId>>::Balance, ValueQuery>;

	/// SP1 verification key hash — updatable via governance when the SP1 program changes
	#[pallet::storage]
	pub type Sp1VkeyHash<T: Config> =
		StorageValue<_, alloc::vec::Vec<u8>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		ProofNotNeeded,
		AlreadyProven,
		RingBufferFull,
		RewardFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProofSubmitted {
			prover: T::AccountId,
			relay_chain_height: u64,
			parachain_height: u64,
			validator_set_id: u64,
			mandatory: bool,
		},
		ProofRewardUpdated {
			new_reward: <T::Currency as Inspect<T::AccountId>>::Balance,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::submit_proof())]
		pub fn submit_proof(
			origin: OriginFor<T>,
			consensus_proof: BoundedVec<u8, T::MaxProofSize>,
			relay_chain_height: u64,
			parachain_height: u64,
			validator_set_id: u64,
		) -> DispatchResult {
			let prover = ensure_signed(origin)?;

			let current_epoch = CurrentEpoch::<T>::get();
			let is_mandatory = validator_set_id > current_epoch;

			if !is_mandatory {
				let last_message = LatestMessageBlock::<T>::get();
				let last_proven = LatestProvenParachainHeight::<T>::get();

				ensure!(
					last_message > last_proven && parachain_height >= last_message,
					Error::<T>::ProofNotNeeded,
				);
			}

			ensure!(
				!ProvenHeights::<T>::contains_key(relay_chain_height),
				Error::<T>::AlreadyProven,
			);

			let trusted_state = ConsensusState::<T>::get();
			let new_state = T::ProofVerifier::verify(&trusted_state, &consensus_proof)?;
			ConsensusState::<T>::put(new_state);

			let offchain_key = Self::offchain_proof_key(relay_chain_height, validator_set_id);
			sp_io::offchain_index::set(&offchain_key, &consensus_proof);

			let metadata = ProofMetadata {
				finalized_height: relay_chain_height,
				validator_set_id,
				prover: prover.clone(),
				created_at: frame_system::Pallet::<T>::block_number(),
			};
			ProvenHeights::<T>::insert(relay_chain_height, metadata.clone());

			RecentProofs::<T>::try_mutate(|proofs| -> DispatchResult {
				if proofs.len() as u32 == T::MaxStoredProofs::get() {
					proofs.remove(0);
				}
				proofs.try_push(metadata).map_err(|_| Error::<T>::RingBufferFull)?;
				Ok(())
			})?;

			LatestProvenParachainHeight::<T>::put(parachain_height);
			if is_mandatory {
				CurrentEpoch::<T>::put(validator_set_id);
			}

			UnprovenEpochs::<T>::remove(validator_set_id);
			UnprovenHeights::<T>::remove(relay_chain_height);

			let reward = ProofReward::<T>::get();
			if reward > Default::default() {
				let treasury: T::AccountId = T::TreasuryPalletId::get().into_account_truncating();
				T::Currency::transfer(&treasury, &prover, reward, Preservation::Preserve)
					.map_err(|_| Error::<T>::RewardFailed)?;
			}

			Self::deposit_event(Event::ProofSubmitted {
				prover,
				relay_chain_height,
				parachain_height,
				validator_set_id,
				mandatory: is_mandatory,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_proof_reward())]
		pub fn set_proof_reward(
			origin: OriginFor<T>,
			reward: <T::Currency as Inspect<T::AccountId>>::Balance,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ProofReward::<T>::put(reward);
			Self::deposit_event(Event::ProofRewardUpdated { new_reward: reward });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_sp1_vkey_hash())]
		pub fn set_sp1_vkey_hash(
			origin: OriginFor<T>,
			vkey_hash: alloc::vec::Vec<u8>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			Sp1VkeyHash::<T>::put(vkey_hash);
			Ok(())
		}
	}

	impl<T: Config> pallet_ismp::OnDispatch for Pallet<T>
	where
		BlockNumberFor<T>: Into<u64>,
	{
		fn on_dispatch() {
			let block: u64 = frame_system::Pallet::<T>::block_number().into();
			LatestMessageBlock::<T>::put(block);
		}
	}

	impl<T: Config> Pallet<T> {
		fn offchain_proof_key(finalized_height: u64, validator_set_id: u64) -> alloc::vec::Vec<u8> {
			let mut key = b"outbound_proofs::".to_vec();
			key.extend_from_slice(&finalized_height.to_be_bytes());
			key.extend_from_slice(&validator_set_id.to_be_bytes());
			key
		}
	}
}
