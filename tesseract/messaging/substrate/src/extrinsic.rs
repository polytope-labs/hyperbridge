// Copyright (C) 2023 Polytope Labs.
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

//! Extrinsic utilities

use anyhow::{anyhow, Context};
use codec::{Decode, Encode};
use subxt::{
	backend::{chain_head::rpc_methods::DryRunResultBytes, legacy::LegacyRpcMethods},
	config::HashFor,
	ext::{scale_decode::DecodeAsType, scale_encode::EncodeAsType},
	tx::{Payload, TxInBlock, TxProgress, TxStatus},
	utils::{MultiSignature, H256},
	OnlineClient,
};

use subxt_utils::refine_subxt_error;
pub use subxt_utils::InMemorySigner;

#[derive(Decode, Encode, DecodeAsType, EncodeAsType, Clone, Debug, Eq, PartialEq)]
#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
pub struct RequestResponseHandled {
	pub commitment: H256,
	pub relayer: Vec<u8>,
}

#[derive(Decode, Encode, DecodeAsType, EncodeAsType, Clone, Debug, Eq, PartialEq)]
#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
pub struct PostRequestHandledEvent(pub RequestResponseHandled);
impl subxt::events::StaticEvent for PostRequestHandledEvent {
	const PALLET: &'static str = "Ismp";
	const EVENT: &'static str = "PostRequestHandled";
}

#[derive(Decode, Encode, DecodeAsType, EncodeAsType, Clone, Debug, Eq, PartialEq)]
#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
pub struct PostResponseHandledEvent(pub RequestResponseHandled);
impl subxt::events::StaticEvent for PostResponseHandledEvent {
	const PALLET: &'static str = "Ismp";
	const EVENT: &'static str = "PostResponseHandled";
}

/// Send an unsigned extrinsic for ISMP messages.
pub async fn send_unsigned_extrinsic<T: subxt::Config, Tx: Payload>(
	client: &OnlineClient<T>,
	payload: Tx,
	wait_for_finalization: bool,
) -> Result<Option<(HashFor<T>, Vec<H256>)>, anyhow::Error>
where
	T::Signature: From<MultiSignature> + Send + Sync,
{
	let ext = client.tx().create_unsigned(&payload)?;
	let progress = match ext.submit_and_watch().await {
		Ok(p) => {
			log::info!(
				"Unsigned extrinsic successfully inserted into pool with hash: {:?}",
				p.extrinsic_hash()
			);

			p
		},
		Err(err) => Err(refine_subxt_error(err)).context("Failed to submit unsigned extrinsic")?,
	};
	let ext_hash = progress.extrinsic_hash();

	let tx_in_block = if wait_for_finalization {
		progress.wait_for_finalized().await.map_err(|err| {
			refine_subxt_error(err).context(format!(
				"Error waiting for unsigned extrinsic in block with hash {ext_hash:?}"
			))
		})
	} else {
		wait_for_inblock::<T>(progress).await
	};

	let extrinsic = match tx_in_block {
		Ok(p) => p,
		Err(err) => Err(err)?,
	};

	let block_hash = extrinsic.block_hash();

	let (hash, receipts) = match extrinsic.wait_for_success().await {
		Ok(p) => {
			log::trace!(target: "tesseract", "Successfully executed unsigned extrinsic {ext_hash:?}");
			let mut receipts = p
				.find::<PostRequestHandledEvent>()
				.filter_map(|ev| ev.ok().map(|e| e.0.commitment.0.into()))
				.collect::<Vec<_>>();
			let temp_2 = p
				.find::<PostResponseHandledEvent>()
				.filter_map(|ev| ev.ok().map(|e| e.0.commitment.0.into()))
				.collect::<Vec<H256>>();
			receipts.extend(temp_2);
			(block_hash, receipts)
		},
		Err(err) => {
			log::trace!(target: "tesseract", "extrinsic execution failed {:?}", err);
			Err(refine_subxt_error(err))
				.context(format!("Error executing unsigned extrinsic {ext_hash:?}"))?
		},
	};
	Ok(Some((hash, receipts)))
}

pub async fn wait_for_inblock<T: subxt::Config>(
	mut progress: TxProgress<T, OnlineClient<T>>,
) -> Result<TxInBlock<T, OnlineClient<T>>, anyhow::Error> {
	let ext_hash = progress.extrinsic_hash();
	while let Some(status) = progress.next().await {
		match status? {
			// Finalized! Return.
			TxStatus::InFinalizedBlock(s) => return Ok(s),
			TxStatus::InBestBlock(s) => return Ok(s),
			// Error scenarios; return the error.
			TxStatus::Error { .. } =>
				return Err(anyhow!(
					"Error waiting for unsigned extrinsic in block with hash {ext_hash:?}"
				)),
			TxStatus::Invalid { .. } => {
				return Err(anyhow!(
					"Error waiting for unsigned extrinsic in block with hash {ext_hash:?}"
				));
			},
			TxStatus::Dropped { .. } => {
				return Err(anyhow!(
					"Error waiting for unsigned extrinsic in block with hash {ext_hash:?}"
				));
			},
			// Ignore and wait for next status event:
			_ => continue,
		}
	}
	Err(anyhow!("Subscription Dropped"))
}
/// Dry run extrinsic
pub async fn system_dry_run_unsigned<T: subxt::Config, Tx: Payload>(
	client: &OnlineClient<T>,
	rpc: &LegacyRpcMethods<T>,
	payload: Tx,
) -> Result<DryRunResultBytes, anyhow::Error>
where
	T::Signature: From<MultiSignature> + Send + Sync,
{
	let ext = client.tx().create_unsigned(&payload)?;
	let result = rpc.dry_run(ext.encoded(), None).await?;
	Ok(result)
}
