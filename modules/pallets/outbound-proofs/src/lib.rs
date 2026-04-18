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
pub use types::ProofMetadata;

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

/// Result of verifying a BEEFY consensus proof. Contains the data extracted from the proof.
#[derive(Clone, Debug)]
pub struct VerifiedProof {
	/// The relay chain height proven by this proof
	pub relay_chain_height: u32,
	/// The parachain block height extracted from the verified parachain header
	pub parachain_height: u32,
	/// The validator set id from the new consensus state
	pub new_validator_set_id: u64,
	/// The validator set id from the trusted (old) consensus state
	pub old_validator_set_id: u64,
	/// The new consensus state bytes to be stored
	pub new_consensus_state: alloc::vec::Vec<u8>,
}

/// Verifies BEEFY consensus proofs and extracts data from them.
pub trait ProofVerifier {
	fn verify_and_extract(
		trusted_state: &[u8],
		proof: &[u8],
	) -> Result<VerifiedProof, frame_support::pallet_prelude::DispatchError>;
}

/// Production BEEFY proof verifier that routes by proof type byte:
/// - `0x00` (PROOF_TYPE_NAIVE): naive verification with ECDSA signature recovery
/// - `0x01` (PROOF_TYPE_SP1): SP1 PLONK ZK proof verification
pub struct BeefyProofVerifier<T>(core::marker::PhantomData<T>);

impl<T: pallet::Config> ProofVerifier for BeefyProofVerifier<T> {
	fn verify_and_extract(
		trusted_state_bytes: &[u8],
		proof: &[u8],
	) -> Result<VerifiedProof, frame_support::pallet_prelude::DispatchError> {
		use codec::Decode;
		use frame_support::{pallet_prelude::DispatchError, traits::Get};
		use ismp::host::StateMachine;
		use sp_runtime::{
			generic::Header,
			traits::{BlakeTwo256, Header as _},
		};

		let trusted_state: beefy_verifier_primitives::ConsensusState =
			codec::Decode::decode(&mut &trusted_state_bytes[..])
				.map_err(|_| DispatchError::Other("Failed to decode consensus state"))?;

		let proof_type = proof.first().ok_or(DispatchError::Other("Empty proof"))?;
		let payload = &proof[1..];

		let (new_state_bytes, verified_parachains) = match *proof_type {
			beefy_verifier_primitives::PROOF_TYPE_NAIVE => {
				let consensus_proof: beefy_verifier_primitives::ConsensusMessage =
					codec::Decode::decode(&mut &payload[..])
						.map_err(|_| DispatchError::Other("Failed to decode naive proof"))?;
				beefy_verifier::verify_consensus::<SubstrateCrypto>(
					trusted_state.clone(),
					consensus_proof,
				)
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
					trusted_state.clone(),
					sp1_proof,
					vkey_hash,
				)
				.map_err(|_| DispatchError::Other("SP1 proof verification failed"))?
			},
			_ => return Err(DispatchError::Other("Unknown proof type")),
		};

		let new_state: beefy_verifier_primitives::ConsensusState =
			codec::Decode::decode(&mut &new_state_bytes[..])
				.map_err(|_| DispatchError::Other("Failed to decode new consensus state"))?;

		let host = <T as pallet_ismp::Config>::HostStateMachine::get();
		let our_para_id = match host {
			StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
			_ => return Err(DispatchError::Other("Host is not a parachain")),
		};

		let mut parachain_height: u32 = 0;
		for para_header in &verified_parachains {
			if para_header.para_id == our_para_id {
				let header = Header::<u32, BlakeTwo256>::decode(&mut &*para_header.header)
					.map_err(|_| DispatchError::Other("Failed to decode parachain header"))?;
				parachain_height = *header.number();
				break;
			}
		}

		Ok(VerifiedProof {
			relay_chain_height: new_state.latest_beefy_height,
			parachain_height,
			new_validator_set_id: new_state.next_authorities.id,
			old_validator_set_id: trusted_state.next_authorities.id,
			new_consensus_state: new_state_bytes,
		})
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

	/// BEEFY consensus client ID for reading state from pallet-ismp
	pub const BEEFY_CONSENSUS_ID: ismp::consensus::ConsensusClientId = *b"BEEF";

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
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

	/// Bounded ring buffer of recent proofs to be polled
	#[pallet::storage]
	pub type RecentProofs<T: Config> = StorageValue<
		_,
		BoundedVec<ProofMetadata<T::AccountId, BlockNumberFor<T>>, T::MaxStoredProofs>,
		ValueQuery,
	>;

	/// The last parachain block number where an ISMP message was dispatched
	#[pallet::storage]
	pub type LatestMessageBlock<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Reward amount per valid proof — updatable via governance
	#[pallet::storage]
	pub type ProofReward<T: Config> =
		StorageValue<_, <<T as Config>::Currency as Inspect<T::AccountId>>::Balance, ValueQuery>;

	/// SP1 verification key hash
	#[pallet::storage]
	pub type Sp1VkeyHash<T: Config> = StorageValue<_, alloc::vec::Vec<u8>, ValueQuery>;

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
			relay_chain_height: u32,
			parachain_height: u32,
			validator_set_id: u64,
			mandatory: bool,
		},
		ProofRewardUpdated {
			new_reward: <<T as Config>::Currency as Inspect<T::AccountId>>::Balance,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::submit_proof())]
		pub fn submit_proof(
			origin: OriginFor<T>,
			consensus_proof: BoundedVec<u8, T::MaxProofSize>,
		) -> DispatchResult {
			let prover = ensure_signed(origin)?;

			// Read the trusted consensus state from pallet-ismp and verify the proof
			let trusted_bytes = pallet_ismp::ConsensusStates::<T>::get(BEEFY_CONSENSUS_ID)
				.ok_or(DispatchError::Other("BEEFY consensus state not initialized"))?;

			let verified =
				<T as Config>::ProofVerifier::verify_and_extract(&trusted_bytes, &consensus_proof)?;

			// Advance the light client, store the new consensus state
			pallet_ismp::ConsensusStates::<T>::insert(
				BEEFY_CONSENSUS_ID,
				verified.new_consensus_state,
			);

			let relay_chain_height = verified.relay_chain_height;
			let parachain_height = verified.parachain_height;
			let is_mandatory = verified.new_validator_set_id > verified.old_validator_set_id;

			if !is_mandatory {
				let last_message = LatestMessageBlock::<T>::get();

				ensure!(
					last_message > 0 && parachain_height as u64 >= last_message,
					Error::<T>::ProofNotNeeded,
				);
			}

			// Check if this relay height has already been proven
			let already_proven = RecentProofs::<T>::get()
				.iter()
				.any(|p| p.finalized_height == relay_chain_height);
			ensure!(!already_proven, Error::<T>::AlreadyProven);

			// Store proof bytes in offchain storage
			let offchain_key =
				Self::offchain_proof_key(relay_chain_height, verified.new_validator_set_id);
			sp_io::offchain_index::set(&offchain_key, &consensus_proof);

			let metadata = ProofMetadata {
				finalized_height: relay_chain_height,
				validator_set_id: verified.new_validator_set_id,
				prover: prover.clone(),
				created_at: frame_system::Pallet::<T>::block_number(),
			};

			RecentProofs::<T>::try_mutate(|proofs| -> DispatchResult {
				if proofs.len() as u32 == <T as Config>::MaxStoredProofs::get() {
					proofs.remove(0);
				}
				proofs.try_push(metadata).map_err(|_| Error::<T>::RingBufferFull)?;
				Ok(())
			})?;

			let reward = ProofReward::<T>::get();
			if reward > Default::default() {
				let treasury: T::AccountId =
					<T as Config>::TreasuryPalletId::get().into_account_truncating();
				<T as Config>::Currency::transfer(
					&treasury,
					&prover,
					reward,
					Preservation::Preserve,
				)
				.map_err(|_| Error::<T>::RewardFailed)?;
			}

			Self::deposit_event(Event::ProofSubmitted {
				prover,
				relay_chain_height,
				parachain_height,
				validator_set_id: verified.new_validator_set_id,
				mandatory: is_mandatory,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_proof_reward())]
		pub fn set_proof_reward(
			origin: OriginFor<T>,
			reward: <<T as Config>::Currency as Inspect<T::AccountId>>::Balance,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
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
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
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
		fn offchain_proof_key(finalized_height: u32, validator_set_id: u64) -> alloc::vec::Vec<u8> {
			let mut key = b"outbound_proofs::".to_vec();
			key.extend_from_slice(&finalized_height.to_be_bytes());
			key.extend_from_slice(&validator_set_id.to_be_bytes());
			key
		}
	}
}
