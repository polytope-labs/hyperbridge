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
#![allow(dead_code)]

use cumulus_primitives_core::CollectCollationInfo;
use pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi;
use polkadot_sdk::*;
use sp_api::{ApiExt, Metadata};
use sp_block_builder::BlockBuilder;
use sp_consensus_aura::{sr25519, AuraApi};
use sp_core::H256;
use sp_offchain::OffchainWorkerApi;
use sp_session::SessionKeys;
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use substrate_frame_rpc_system::AccountNonceApi;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{
		generic,
		traits::{Hash as HashT, Keccak256},
		MultiAddress, MultiSignature,
	};

	use sp_runtime::traits::{IdentifyAccount, Verify};
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, Keccak256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <Keccak256 as HashT>::Output;

	/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
	pub type Signature = MultiSignature;

	/// Some way of identifying an account on the chain. We intentionally make it equivalent
	/// to the public key of our transaction signing scheme.
	pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

	/// Balance of an account.
	pub type Balance = u128;

	/// Index of a transaction in the chain.
	pub type Index = u32;

	/// An index to a block.
	pub type BlockNumber = u32;

	/// The address format for describing accounts.
	pub type Address = MultiAddress<AccountId, ()>;
}

pub trait BaseHostRuntimeApis:
	TaggedTransactionQueue<opaque::Block>
	+ ApiExt<opaque::Block>
	+ BlockBuilder<opaque::Block>
	+ AccountNonceApi<opaque::Block, opaque::AccountId, opaque::Index>
	+ Metadata<opaque::Block>
	+ AuraApi<opaque::Block, sr25519::AuthorityId>
	+ OffchainWorkerApi<opaque::Block>
	+ SessionKeys<opaque::Block>
	+ CollectCollationInfo<opaque::Block>
	+ TransactionPaymentRuntimeApi<opaque::Block, opaque::Balance>
	+ ismp_parachain_runtime_api::IsmpParachainApi<opaque::Block>
	+ pallet_ismp_runtime_api::IsmpRuntimeApi<opaque::Block, H256>
	+ cumulus_primitives_aura::AuraUnincludedSegmentApi<opaque::Block>
	+ pallet_mmr_runtime_api::MmrRuntimeApi<
		opaque::Block,
		H256,
		opaque::BlockNumber,
		pallet_ismp::offchain::Leaf,
	> + simnode_runtime_api::CreateTransactionApi<
		opaque::Block,
		gargantua_runtime::RuntimeCall,
		opaque::AccountId,
	>
{
}

impl<Api> BaseHostRuntimeApis for Api where
	Api: TaggedTransactionQueue<opaque::Block>
		+ ApiExt<opaque::Block>
		+ BlockBuilder<opaque::Block>
		+ AccountNonceApi<opaque::Block, opaque::AccountId, opaque::Index>
		+ Metadata<opaque::Block>
		+ AuraApi<opaque::Block, sr25519::AuthorityId>
		+ OffchainWorkerApi<opaque::Block>
		+ SessionKeys<opaque::Block>
		+ CollectCollationInfo<opaque::Block>
		+ TransactionPaymentRuntimeApi<opaque::Block, opaque::Balance>
		+ ismp_parachain_runtime_api::IsmpParachainApi<opaque::Block>
		+ pallet_ismp_runtime_api::IsmpRuntimeApi<opaque::Block, H256>
		+ cumulus_primitives_aura::AuraUnincludedSegmentApi<opaque::Block>
		+ pallet_mmr_runtime_api::MmrRuntimeApi<
			opaque::Block,
			H256,
			opaque::BlockNumber,
			pallet_ismp::offchain::Leaf,
		> + simnode_runtime_api::CreateTransactionApi<
			opaque::Block,
			gargantua_runtime::RuntimeCall,
			opaque::AccountId,
		>
{
}
