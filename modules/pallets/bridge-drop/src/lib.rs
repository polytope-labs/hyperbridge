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

//! Airdrop for Bridge Tokens

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub use pallet::*;
use polkadot_sdk::*;

/// Eighteen months in hyperbridge blocks
pub const EIGHTEEN_MONTHS: u64 = 3_888_000;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{format, vec, vec::Vec};
	use anyhow::anyhow;
	use codec::Encode;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use polkadot_sdk::{
		frame_support::{
			traits::{Currency, ExistenceRequirement, VestingSchedule},
			PalletId,
		},
		frame_system::ensure_none,
		sp_core::{H160, H256},
		sp_runtime::{traits::AccountIdConversion, Permill},
	};
	use rs_merkle::MerkleProof;

	type VestingBalanceOf<T> = <<T as pallet_vesting::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ polkadot_sdk::pallet_balances::Config
		+ pallet_ismp::Config
		+ pallet_vesting::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		/// Currency implementation
		type Currency: Currency<Self::AccountId>;
	}

	/// Set of leaf indexes that have been claimed
	#[pallet::storage]
	#[pallet::getter(fn claimed)]
	pub type Claimed<T: Config> = StorageMap<_, Blake2_128Concat, u64, bool, OptionQuery>;

	/// Set of leaf indexes that have been claimed
	#[pallet::storage]
	#[pallet::getter(fn iro_claimed)]
	pub type IroClaimed<T: Config> = StorageMap<_, Blake2_128Concat, u64, bool, OptionQuery>;

	/// Set of leaf indexes that have been claimed
	#[pallet::storage]
	#[pallet::getter(fn crowdloan_claimed)]
	pub type CrowdloanClaimed<T: Config> = StorageMap<_, Blake2_128Concat, u64, bool, OptionQuery>;

	/// Merkle root and total leafcount
	#[pallet::storage]
	#[pallet::getter(fn merkle_root)]
	pub type MerkleRoot<T: Config> = StorageValue<_, (H256, u64), OptionQuery>;

	/// Merkle root and total leafcount
	#[pallet::storage]
	#[pallet::getter(fn iro_merkle_root)]
	pub type IroMerkleRoot<T: Config> = StorageValue<_, (H256, u64), OptionQuery>;

	/// Merkle root and total leafcount
	#[pallet::storage]
	#[pallet::getter(fn crowdloan_merkle_root)]
	pub type CrowdloanMerkleRoot<T: Config> = StorageValue<_, (H256, u64), OptionQuery>;
	/// Merkle root and total leafcount
	#[pallet::storage]
	#[pallet::getter(fn starting_block)]
	pub type StartingBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Account has claimed the airdrop already
		AlreadyClaimed,
		/// Invalid claim merkle proof
		InvalidProof,
		/// Pallet has not been initialized
		MerkleRootNotFound,
		/// Invalid Leaf Index
		InvalidLeafIndex,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Airdrop claimed
		Claimed {
			beneficiary: T::AccountId,
			amount: <<T as Config>::Currency as Currency<T::AccountId>>::Balance,
		},
	}

	#[derive(
		Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct Proof<AccountId, Balance> {
		/// Account Eligible for the claim
		pub who: H160,
		/// Receiving account on Hyperbridge
		pub beneficiary: AccountId,
		/// Signature that approves the receiving address
		pub signature: Vec<u8>,
		/// Merkle proof of eligibility
		pub proof_items: Vec<H256>,
		/// Leaf index for (who, amount) in the merkle tree
		pub leaf_index: u64,
		/// Amount to claim
		pub amount: Balance,
	}

	#[derive(
		Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
	)]
	pub struct IroProof<AccountId, Balance> {
		/// Receiving account on Hyperbridge
		pub beneficiary: AccountId,
		/// Merkle proof of eligibility
		pub proof_items: Vec<H256>,
		/// Leaf index for (beneficiary, amount) in the merkle tree
		pub leaf_index: u64,
		/// Amount to claim
		pub amount: Balance,
	}

	#[derive(Clone, Copy)]
	pub struct KeccakHasher;

	impl rs_merkle::Hasher for KeccakHasher {
		type Hash = [u8; 32];

		fn hash(data: &[u8]) -> Self::Hash {
			sp_io::hashing::keccak_256(data)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			if StartingBlock::<T>::get().is_none() {
				StartingBlock::<T>::put(n);
				return <T as frame_system::Config>::DbWeight::get().reads_writes(1, 1);
			}

			<T as frame_system::Config>::DbWeight::get().reads(1)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<<T as Config>::Currency as Currency<T::AccountId>>::Balance: From<u128>,
		u128: From<<<T as Config>::Currency as Currency<T::AccountId>>::Balance>,
		VestingBalanceOf<T>: From<u128>,
	{
		/// Set merkle root for claims
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn set_airdrop_merkle_root(
			origin: OriginFor<T>,
			root: H256,
			leaf_count: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			MerkleRoot::<T>::put((root, leaf_count));
			Ok(())
		}

		/// Set merkle root for iro
		#[pallet::call_index(1)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn set_iro_merkle_root(
			origin: OriginFor<T>,
			root: H256,
			leaf_count: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			IroMerkleRoot::<T>::put((root, leaf_count));
			Ok(())
		}

		/// Set merkle root for crowdloan claims
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn set_crowdloan_merkle_root(
			origin: OriginFor<T>,
			root: H256,
			leaf_count: u64,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			CrowdloanMerkleRoot::<T>::put((root, leaf_count));
			Ok(())
		}

		/// Claim bridge tokens
		#[pallet::call_index(3)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn claim_tokens(
			origin: OriginFor<T>,
			params: Proof<
				T::AccountId,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			ensure_none(origin)?;

			let beneficiary = params.beneficiary.clone();
			let amount = params.amount;
			Self::execute_claim(params)?;

			// Unlock 25% of token amount
			let percent = Permill::from_parts(250_000);
			let unlocked_balance = percent * u128::from(amount);

			let locked = u128::from(amount).saturating_sub(unlocked_balance);

			<<T as Config>::Currency as Currency<T::AccountId>>::transfer(
				&Self::account_id(),
				&beneficiary,
				amount.into(),
				ExistenceRequirement::AllowDeath,
			)?;

			let unlock_per_block = locked / EIGHTEEN_MONTHS as u128;

			let starting_block =
				StartingBlock::<T>::get().unwrap_or(frame_system::Pallet::<T>::block_number());

			pallet_vesting::Pallet::<T>::add_vesting_schedule(
				&beneficiary,
				locked.into(),
				unlock_per_block.into(),
				starting_block,
			)?;

			Self::deposit_event(Event::<T>::Claimed { beneficiary, amount });

			Ok(())
		}

		/// Claim iro tokens
		#[pallet::call_index(4)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn claim_iro(
			origin: OriginFor<T>,
			params: IroProof<
				T::AccountId,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			ensure_none(origin)?;

			let beneficiary = params.beneficiary.clone();
			let amount = params.amount;
			Self::execute_iro_claim(params)?;

			// Unlock 25% of token amount
			let percent = Permill::from_parts(250_000);
			let unlocked_balance = percent * u128::from(amount);

			let locked = u128::from(amount).saturating_sub(unlocked_balance);

			<<T as Config>::Currency as Currency<T::AccountId>>::transfer(
				&Self::account_id(),
				&beneficiary,
				amount.into(),
				ExistenceRequirement::AllowDeath,
			)?;

			let unlock_per_block = locked / EIGHTEEN_MONTHS as u128;

			let starting_block =
				StartingBlock::<T>::get().unwrap_or(frame_system::Pallet::<T>::block_number());

			pallet_vesting::Pallet::<T>::add_vesting_schedule(
				&beneficiary,
				locked.into(),
				unlock_per_block.into(),
				starting_block,
			)?;

			Self::deposit_event(Event::<T>::Claimed { beneficiary, amount });

			Ok(())
		}

		/// Claim crowdloan tokens
		#[pallet::call_index(5)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn claim_crowdloan(
			origin: OriginFor<T>,
			params: IroProof<
				T::AccountId,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			ensure_none(origin)?;

			let beneficiary = params.beneficiary.clone();
			let amount = u128::from(params.amount);
			Self::execute_crowdloan_claim(params)?;

			<<T as Config>::Currency as Currency<T::AccountId>>::transfer(
				&Self::account_id(),
				&beneficiary,
				amount.into(),
				ExistenceRequirement::AllowDeath,
			)?;

			let unlock_per_block = amount / EIGHTEEN_MONTHS as u128;

			let starting_block =
				StartingBlock::<T>::get().unwrap_or(frame_system::Pallet::<T>::block_number());

			pallet_vesting::Pallet::<T>::add_vesting_schedule(
				&beneficiary,
				amount.into(),
				unlock_per_block.into(),
				starting_block,
			)?;

			Self::deposit_event(Event::<T>::Claimed { beneficiary, amount: amount.into() });

			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		<<T as Config>::Currency as Currency<T::AccountId>>::Balance: From<u128>,
		u128: From<<<T as Config>::Currency as Currency<T::AccountId>>::Balance>,
		VestingBalanceOf<T>: From<u128>,
	{
		type Call = Call<T>;

		// empty pre-dispatch so we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let res = match call {
				Call::claim_tokens { params } => Self::execute_claim(params.clone())
					.map(|_| sp_io::hashing::keccak_256(&params.encode())),
				Call::claim_iro { params } => Self::execute_iro_claim(params.clone())
					.map(|_| sp_io::hashing::keccak_256(&params.encode())),
				_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
			};

			let msg_hash = match res {
				Ok(msg_hash) => msg_hash,
				Err(_) => {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
				},
			};

			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides: vec![msg_hash.to_vec()],
				longevity: 25,
				propagate: true,
			})
		}
	}

	impl<T: Config> Pallet<T> {
		fn execute_claim(
			params: Proof<
				T::AccountId,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			let (root, leaf_count) =
				MerkleRoot::<T>::get().ok_or_else(|| Error::<T>::MerkleRootNotFound)?;

			if Claimed::<T>::get(params.leaf_index).is_some() {
				Err(Error::<T>::AlreadyClaimed)?
			}

			if params.leaf_index >= leaf_count {
				Err(Error::<T>::InvalidLeafIndex)?
			}

			verify_proof(root, leaf_count, params.clone()).map_err(|_| Error::<T>::InvalidProof)?;

			Claimed::<T>::insert(params.leaf_index, true);
			Ok(())
		}

		fn execute_iro_claim(
			params: IroProof<
				T::AccountId,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			let (root, leaf_count) =
				IroMerkleRoot::<T>::get().ok_or_else(|| Error::<T>::MerkleRootNotFound)?;

			if IroClaimed::<T>::get(params.leaf_index).is_some() {
				Err(Error::<T>::AlreadyClaimed)?
			}

			if params.leaf_index >= leaf_count {
				Err(Error::<T>::InvalidLeafIndex)?
			}

			verify_iro_proof(root, leaf_count, params.clone())
				.map_err(|_| Error::<T>::InvalidProof)?;

			IroClaimed::<T>::insert(params.leaf_index, true);
			Ok(())
		}

		fn execute_crowdloan_claim(
			params: IroProof<
				T::AccountId,
				<<T as Config>::Currency as Currency<T::AccountId>>::Balance,
			>,
		) -> DispatchResult {
			let (root, leaf_count) =
				CrowdloanMerkleRoot::<T>::get().ok_or_else(|| Error::<T>::MerkleRootNotFound)?;

			if CrowdloanClaimed::<T>::get(params.leaf_index).is_some() {
				Err(Error::<T>::AlreadyClaimed)?
			}

			if params.leaf_index >= leaf_count {
				Err(Error::<T>::InvalidLeafIndex)?
			}

			verify_iro_proof(root, leaf_count, params.clone())
				.map_err(|_| Error::<T>::InvalidProof)?;

			CrowdloanClaimed::<T>::insert(params.leaf_index, true);
			Ok(())
		}

		/// Account that should hold all tokens for airdrop
		pub fn account_id() -> T::AccountId {
			PalletId(*b"BRIDGE//").into_account_truncating()
		}
	}

	pub const ETHEREUM_MESSAGE_PREFIX: &'static str = "\x19Ethereum Signed Message:\n";
	fn verify_proof<AccountId: Encode, Balance: Encode>(
		merkle_root: H256,
		leaf_count: u64,
		params: Proof<AccountId, Balance>,
	) -> Result<(), anyhow::Error> {
		// Verify signature

		if params.signature.len() != 65 {
			Err(anyhow!("Invalid Signature"))?
		}

		let mut signature = [0u8; 65];

		let payload = params.beneficiary.encode();

		signature.copy_from_slice(&params.signature);

		// Following EIP-191 convention https://eips.ethereum.org/EIPS/eip-191
		let preimage = vec![
			format!("{ETHEREUM_MESSAGE_PREFIX}{}", payload.len()).as_bytes().to_vec(),
			payload,
		]
		.concat();
		let message = sp_io::hashing::keccak_256(&preimage);
		let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &message)
			.map_err(|_| anyhow!("Failed to recover ecdsa public key from signature"))?;
		let eth_address = H160::from_slice(&sp_io::hashing::keccak_256(&pub_key[..])[12..]);

		if eth_address != params.who {
			Err(anyhow!("Invalid Signature"))?
		}

		let proof = MerkleProof::<KeccakHasher>::new(
			params.proof_items.into_iter().map(|val| val.0).collect(),
		);

		let leaf_hash = sp_io::hashing::keccak_256(&(params.who, params.amount).encode());

		if !proof.verify(
			merkle_root.0,
			&[params.leaf_index as usize],
			&[leaf_hash],
			leaf_count as usize,
		) {
			Err(anyhow!("Invalid Merkle Proof"))?
		}

		Ok(())
	}

	fn verify_iro_proof<AccountId: Encode, Balance: Encode>(
		merkle_root: H256,
		leaf_count: u64,
		params: IroProof<AccountId, Balance>,
	) -> Result<(), anyhow::Error> {
		let proof = MerkleProof::<KeccakHasher>::new(
			params.proof_items.into_iter().map(|val| val.0).collect(),
		);

		let leaf_hash = sp_io::hashing::keccak_256(&(params.beneficiary, params.amount).encode());

		if !proof.verify(
			merkle_root.0,
			&[params.leaf_index as usize],
			&[leaf_hash],
			leaf_count as usize,
		) {
			Err(anyhow!("Invalid Merkle Proof"))?
		}

		Ok(())
	}
}
