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

//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use polkadot_sdk::*;
use std::sync::Arc;

use gargantua_runtime::{opaque::Block, AccountId, Balance, Index as Nonce};

use crate::runtime_api::opaque;
use sc_client_api::{AuxStore, BlockBackend, ProofProvider};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::H256;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, P, B> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Backend used by the node.
	pub backend: Arc<B>,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, B>(
	deps: FullDeps<C, P, B>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ BlockBackend<Block>
		+ ProofProvider<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: BlockBuilder<Block>,
	C::Api: pallet_ismp_runtime_api::IsmpRuntimeApi<opaque::Block, H256>,
	C::Api: pallet_mmr_runtime_api::MmrRuntimeApi<
		opaque::Block,
		H256,
		opaque::BlockNumber,
		pallet_ismp::offchain::Leaf,
	>,
	P: TransactionPool + Sync + Send + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::StateBackend<sp_runtime::traits::HashingFor<Block>>,
{
	use pallet_ismp_rpc::{IsmpApiServer, IsmpRpcHandler};
	use pallet_mmr_rpc::{MmrApiServer, MmrRpcHandler};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut module = RpcExtension::new(());
	let FullDeps { client, pool, backend } = deps;

	module.merge(System::new(client.clone(), pool).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	module.merge(IsmpRpcHandler::new(client.clone(), backend.clone())?.into_rpc())?;
	module.merge(MmrRpcHandler::new(client, backend.clone())?.into_rpc())?;

	Ok(module)
}
