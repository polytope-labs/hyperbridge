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

pub mod precompile;

pub use pallet::*;
use polkadot_sdk::*;

extern crate alloc;
use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use cumulus_pallet_parachain_system::{RelaychainDataProvider, RelaychainStateProvider};
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use pallet_revive::{H160, H256, inherent_handlers::InherentHandler};
use sp_consensus_aura::{AURA_ENGINE_ID, Slot};
use sp_core::ConstU32;
use sp_io::hashing::{keccak_256, twox_64};
use sp_runtime::{
	DigestItem, DispatchError, DispatchResult,
	app_crypto::sp_core::storage::StorageKey,
	generic::Header,
	traits::{BlakeTwo256, Block as BlockT, Header as HeaderT},
};
use sp_std::time::Duration;
use sp_trie::StorageProof;
use substrate_state_machine::read_proof_check_for_parachain;

/// Hyperbridge's Inherent identifier
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"hypbridg";

/// The `ConsensusEngineId` of ISMP `ConsensusDigest` in the parachain header.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

pub const ISMP_TIMESTAMP_ID: sp_runtime::ConsensusEngineId = *b"ISTM";

/// Timestamp log digest for pallet ismp
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Default)]
pub struct TimestampDigest {
	/// Timestamp value in seconds
	pub timestamp: u64,
}

/// Consensus log digest for pallet ismp
#[derive(Encode, Decode, Clone, scale_info::TypeInfo, Default)]
pub struct ConsensusDigest {
	/// Mmr root hash
	pub mmr_root: H256,
	/// Child trie root hash
	pub child_trie_root: H256,
}

/// Proof for proving finality
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, TypeInfo, DecodeWithMemTracking)]
pub struct HyperbridgeConsensusProof {
	/// Height of the relay chain for the given proof
	pub relay_height: u32,
	/// Storage proof for the parachain headers
	pub storage_proof: Vec<Vec<u8>>,
}

/// Commitment to Hyperbridge State Machine at a given height
#[derive(
	Debug,
	Encode,
	Decode,
	Clone,
	PartialEq,
	Eq,
	DecodeWithMemTracking,
	TypeInfo,
	Default,
	MaxEncodedLen,
)]
pub struct StateCommitment {
	/// Timestamp in seconds
	pub timestamp: u64,
	/// Root hash of the request/response overlay trie if the state machine supports it.
	pub overlay_root: Option<H256>,
	/// Root hash of the global state trie.
	pub state_root: H256,
}

/// Identifies a state commitment at a given height
#[derive(
	Debug,
	Encode,
	Decode,
	Clone,
	PartialEq,
	Eq,
	DecodeWithMemTracking,
	TypeInfo,
	Default,
	MaxEncodedLen,
)]
pub struct StateCommitmentHeight {
	/// The state machine identifier
	pub commitment: StateCommitment,
	/// The corresponding block height
	pub height: u64,
}

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_revive::Config
		+ cumulus_pallet_parachain_system::Config
	{
		/// The specific parachain ID for Hyperbridge that this pallet verifies.
		#[pallet::constant]
		type HyperbridgeParaId: Get<u32>;

		/// The ISMP Host contract address for Hyperbridge
		#[pallet::constant]
		type IsmpHostContractAddress: Get<H160>;
	}

	/// Stores the latest verified StateCommitmentHeight for Hyperbridge.
	#[pallet::storage]
	#[pallet::getter(fn hyperbridge_state_commitment_height)]
	pub type HyperbridgeStateCommitmentHeight<T: Config> =
		StorageValue<_, StateCommitmentHeight, OptionQuery>;

	/// Tracks whether the inherent has run for the current block.
	#[pallet::storage]
	#[pallet::getter(fn inherent_processed)]
	pub type InherentProcessed<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		HyperbridgeProofVerified { commitment: StateCommitmentHeight },
	}

	#[pallet::error]
	pub enum Error<T> {
		InherentAlreadyProcessed,
		StorageProofVerificationFailed,
		HyperbridgeHeaderNotFound,
		HeaderDecodingFailed,
		DigestExtractionError,
		TimestampNotFound,
		HandlerProofDecodingFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Inherent extrinsic to submit and verify the Hyperbridge proof.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::MAX)]
		pub fn submit_hyperbridge_proof(
			origin: OriginFor<T>,
			proof: HyperbridgeConsensusProof,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			ensure!(!InherentProcessed::<T>::get(), Error::<T>::InherentAlreadyProcessed);

			let commitment = Self::verify_store_and_extract(&proof)?;

			Self::deposit_event(Event::HyperbridgeProofVerified { commitment });

			InherentProcessed::<T>::put(true);
			Ok(Pays::No.into())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = sp_inherents::MakeFatalError<()>;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			if InherentProcessed::<T>::get() {
				return None;
			}
			let proof_data: HyperbridgeConsensusProof =
				data.get_data(&Self::INHERENT_IDENTIFIER).ok().flatten()?;
			Some(Call::submit_hyperbridge_proof { proof: proof_data })
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::submit_hyperbridge_proof { .. }) && !InherentProcessed::<T>::get()
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			InherentProcessed::<T>::put(false);
			Weight::zero()
		}
	}

	impl<T: Config> InherentHandler for Pallet<T> {
		fn handler_name() -> &'static [u8] {
			b"hyperbridge_proof_verifier_v1"
		}

		fn handle_message(message: Vec<u8>) -> DispatchResult {
			let proof = HyperbridgeConsensusProof::decode(&mut &message[..])
				.map_err(|_| Error::<T>::HandlerProofDecodingFailed)?;

			let final_commitment = Self::verify_store_and_extract(&proof)?;

			let ismp_host_address = T::IsmpHostContractAddress::get();
			let topic_0_hash = keccak_256(b"StateMachineUpdated(bytes32,bytes)");
			let topic_0: H256 = H256::from(topic_0_hash);

			let topics = vec![topic_0];

			let data = final_commitment.encode();

			<frame_system::Pallet<T>>::deposit_event(
				<T as pallet_revive::Config>::RuntimeEvent::from(
					pallet_revive::Event::ContractEmitted {
						contract: ismp_host_address,
						topics,
						data,
					},
				),
			);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Verifies proof for Hyperbridge, extracts digest info, stores the commitment height.
	fn verify_store_and_extract(
		proof: &HyperbridgeConsensusProof,
	) -> Result<StateCommitmentHeight, DispatchError> {
		let state = RelaychainDataProvider::<T>::current_relay_chain_state();
		let relay_root = state.state_root;

		let storage_proof = StorageProof::new(proof.storage_proof.clone());
		let hyperbridge_para_id = T::HyperbridgeParaId::get();
		let header_key = parachain_header_storage_key(hyperbridge_para_id).0;
		let keys_to_prove = alloc::vec![header_key];

		let read_result = read_proof_check_for_parachain::<BlakeTwo256, _>(
			&relay_root,
			storage_proof,
			keys_to_prove.iter().cloned(),
		)
			.map_err(|e| {
				log::error!(target: "runtime::hyperbridge-verifier", "Storage proof verification failed: {:?}", e);
				Error::<T>::StorageProofVerificationFailed
			})?;

		let header_bytes = read_result
			.get(&keys_to_prove[0])
			.cloned()
			.flatten()
			.ok_or(Error::<T>::HyperbridgeHeaderNotFound)?;

		let decoded_vec: Vec<u8> =
			Decode::decode(&mut &header_bytes[..]).map_err(|_| Error::<T>::HeaderDecodingFailed)?;
		let header = Header::<u32, BlakeTwo256>::decode(&mut &decoded_vec[..])
			.map_err(|_| Error::<T>::HeaderDecodingFailed)?;

		let mut timestamp: u64 = 0;
		let mut overlay_root: H256 = H256::default();
		for digest in header.digest().logs.iter() {
			match digest {
				DigestItem::Consensus(consensus_engine_id, value)
					if *consensus_engine_id == ISMP_TIMESTAMP_ID =>
				{
					let timestamp_digest = TimestampDigest::decode(&mut &value[..])
						.map_err(|_| Error::<T>::DigestExtractionError)?;
					timestamp = timestamp_digest.timestamp;
				},
				DigestItem::Consensus(consensus_engine_id, value)
					if *consensus_engine_id == ISMP_ID =>
				{
					if let Ok(log) = ConsensusDigest::decode(&mut &value[..]) {
						overlay_root = log.child_trie_root;
					} else {
						log::warn!(target: "ismp::hyperbridge-verifier", "Invalid ISMP digest found");
					}
				},
				_ => {},
			};
		}

		ensure!(timestamp > 0, Error::<T>::TimestampNotFound);

		let block_height: u64 = (*header.number()).into();

		let commitment_height = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp,
				overlay_root: Some(overlay_root),
				state_root: *header.state_root(),
			},
			height: block_height,
		};
		HyperbridgeStateCommitmentHeight::<T>::put(commitment_height.clone());

		Ok(commitment_height)
	}
}

pub fn parachain_header_storage_key(para_id: u32) -> StorageKey {
	let mut storage_key = storage::storage_prefix(b"Paras", b"Heads").to_vec();
	let encoded_para_id = para_id.encode();
	storage_key.extend_from_slice(twox_64(&encoded_para_id).as_slice());
	storage_key.extend_from_slice(&encoded_para_id);
	StorageKey(storage_key)
}
