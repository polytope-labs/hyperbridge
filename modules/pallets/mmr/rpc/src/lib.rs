// Copyright (c) 2025 Polytope Labs.
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

use jsonrpsee::{core::RpcResult, proc_macros::rpc};

use anyhow::anyhow;
use codec::Encode;
use pallet_ismp::offchain::{Leaf, ProofKeys};
use pallet_ismp_rpc::{runtime_error_into_rpc_error, Proof};
use pallet_mmr_runtime_api::MmrRuntimeApi;
use polkadot_sdk::*;
use sc_client_api::{Backend, BlockBackend};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_core::{
	offchain::{storage::OffchainDb, OffchainDbExt, OffchainStorage},
	H256,
};
use sp_runtime::traits::{Block as BlockT, Header, NumberFor};
use std::sync::Arc;

/// Mmr RPC methods.
#[rpc(client, server)]
pub trait MmrApi<Hash> {
	/// Query mmr proof for some commitments
	#[method(name = "mmr_queryProof")]
	fn query_proof(&self, height: u32, keys: ProofKeys) -> RpcResult<Proof>;
}

/// An implementation of Mmr specific RPC methods.
pub struct MmrRpcHandler<C, B, S, T> {
	client: Arc<C>,
	offchain_db: OffchainDb<S>,
	_marker: std::marker::PhantomData<(B, T)>,
}

impl<C, B, S, T> MmrRpcHandler<C, B, S, T>
where
	B: BlockT,
	S: OffchainStorage + Clone + Send + Sync + 'static,
	T: Backend<B, OffchainStorage = S> + Send + Sync + 'static,
{
	/// Create new `MmrRpcHandler` with the given reference to the client.
	pub fn new(client: Arc<C>, backend: Arc<T>) -> Result<Self, anyhow::Error> {
		let offchain_db = OffchainDb::new(
			backend
				.offchain_storage()
				.ok_or_else(|| anyhow!("Offchain Storage not present in backend!"))?,
		);

		Ok(Self { client, offchain_db, _marker: Default::default() })
	}
}

impl<C, B, S, T> MmrApiServer<B::Hash> for MmrRpcHandler<C, B, S, T>
where
	B: BlockT,
	B::Hash: Into<H256>,
	S: OffchainStorage + Clone + Send + Sync + 'static,
	T: Backend<B> + Send + Sync + 'static,
	C: Send + Sync + 'static + ProvideRuntimeApi<B> + BlockBackend<B>,
	C::Api: MmrRuntimeApi<B, B::Hash, NumberFor<B>, Leaf>,
	u64: From<<B::Header as Header>::Number>,
{
	fn query_proof(&self, height: u32, keys: ProofKeys) -> RpcResult<Proof> {
		let mut api = self.client.runtime_api();
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let at = self
			.client
			.block_hash(height.into())
			.ok()
			.flatten()
			.ok_or_else(|| runtime_error_into_rpc_error("invalid block height provided"))?;
		let (_, proof): (Vec<Leaf>, pallet_ismp::offchain::Proof<B::Hash>) = api
			.generate_proof(at, keys)
			.map_err(|e| runtime_error_into_rpc_error(format!("Error calling runtime api: {e:?}")))?
			.map_err(|e| {
				runtime_error_into_rpc_error(format!("Error generating mmr proof: {e:?}"))
			})?;
		Ok(Proof { proof: proof.encode(), height })
	}
}
