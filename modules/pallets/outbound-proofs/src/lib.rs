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

mod benchmarking;

use polkadot_sdk::*;

pub use pallet::*;
pub use types::{EpochInfo, PendingProofInfo, ProofMetadata};

/// Verifies BEEFY consensus proofs. The trusted state and proof are raw bytes.
pub trait ProofVerifier {
	fn verify(
		trusted_state: &[u8],
		proof: &[u8],
	) -> Result<alloc::vec::Vec<u8>, frame_support::pallet_prelude::DispatchError>;
}

/// Crypto implementation using substrate host functions for BEEFY proof verification.
pub struct SubstrateCrypto;

impl ismp::messaging::Keccak256 for SubstrateCrypto {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

impl beefy_verifier::EcdsaRecover for SubstrateCrypto {
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]> {
		sp_io::crypto::secp256k1_ecdsa_recover(signature, prehash)
			.map_err(|_| anyhow::anyhow!("Failed to recover secp256k1 public key"))
	}
}

/// Production BEEFY proof verifier that routes by proof type byte:
/// - `0x00` (PROOF_TYPE_NAIVE): naive verification with ECDSA signature recovery
/// - `0x01` (PROOF_TYPE_SP1): SP1 PLONK ZK proof verification
pub struct BeefyProofVerifier<T>(core::marker::PhantomData<T>);

impl<T: pallet::Config> ProofVerifier for BeefyProofVerifier<T> {
	fn verify(
		trusted_state_bytes: &[u8],
		proof: &[u8],
	) -> Result<alloc::vec::Vec<u8>, frame_support::pallet_prelude::DispatchError> {
		use frame_support::pallet_prelude::DispatchError;

		let proof_type = proof.first().ok_or(DispatchError::Other("Empty proof"))?;
		let payload = &proof[1..];

		let trusted_state: beefy_verifier_primitives::ConsensusState =
			codec::Decode::decode(&mut &trusted_state_bytes[..])
				.map_err(|_| DispatchError::Other("Failed to decode consensus state"))?;

		let (new_state_bytes, _headers) = match *proof_type {
			beefy_verifier_primitives::PROOF_TYPE_NAIVE => {
				let consensus_proof: beefy_verifier_primitives::BeefyConsensusProof =
					codec::Decode::decode(&mut &payload[..])
						.map_err(|_| DispatchError::Other("Failed to decode naive proof"))?;
				beefy_verifier::verify_consensus::<SubstrateCrypto>(trusted_state, consensus_proof)
					.map_err(|_| DispatchError::Other("Naive proof verification failed"))?
			},
			beefy_verifier_primitives::PROOF_TYPE_SP1 => {
				let sp1_proof: beefy_verifier_primitives::Sp1BeefyProof =
					codec::Decode::decode(&mut &payload[..])
						.map_err(|_| DispatchError::Other("Failed to decode SP1 proof"))?;
				let vkey_bytes = pallet::Sp1VkeyHash::<T>::get();
				let vkey_hash = core::str::from_utf8(&vkey_bytes)
					.map_err(|_| DispatchError::Other("Invalid SP1 vkey hash encoding"))?;
				beefy_verifier::sp1::verify_sp1_consensus::<SubstrateCrypto>(
					trusted_state,
					sp1_proof,
					vkey_hash,
				)
				.map_err(|_| DispatchError::Other("SP1 proof verification failed"))?
			},
			_ => return Err(DispatchError::Other("Unknown proof type")),
		};

		Ok(new_state_bytes)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
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

	/// BEEFY consensus state — stored as SCALE-encoded bytes of
	/// `beefy_verifier_primitives::ConsensusState`
	#[pallet::storage]
	pub type ConsensusState<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

	/// Reward amount per valid proof — updatable via governance
	#[pallet::storage]
	pub type ProofReward<T: Config> =
		StorageValue<_, <T::Currency as Inspect<T::AccountId>>::Balance, ValueQuery>;

	/// SP1 verification key hash — updatable via governance when the SP1 program changes
	#[pallet::storage]
	pub type Sp1VkeyHash<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

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

			let trusted_bytes = ConsensusState::<T>::get();
			ensure!(
				!trusted_bytes.is_empty(),
				DispatchError::Other("Consensus state not initialized"),
			);

			let new_state_bytes = T::ProofVerifier::verify(&trusted_bytes, &consensus_proof)?;
			ConsensusState::<T>::put(&new_state_bytes);

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
