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

use anyhow::Context;
use codec::{Decode, Encode};
use sp_core::H256;
use subxt::{
	config:: ExtrinsicParams,
	tx::Payload,
	OnlineClient,
};
use subxt::ext::{scale_decode::DecodeAsType, scale_encode::EncodeAsType};
use subxt::ext::subxt_rpcs::methods::legacy::DryRunResult;
use polkadot_sdk::sp_runtime::MultiSignature;
use subxt::config::HashFor;

use subxt_utils::refine_subxt_error;
pub use subxt_utils::{InMemorySigner};

#[derive(Decode, Encode, DecodeAsType, EncodeAsType, Clone, Debug, Eq, PartialEq)]
#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
pub struct RequestResponseHandled {
	pub commitment: primitive_types_old::H256,
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
		progress.wait_for_finalized().await
	} else {
		progress.wait_for_in_block().await
	};

	let extrinsic = match tx_in_block {
		Ok(p) => p,
		Err(err) => Err(refine_subxt_error(err)).context(format!(
			"Error waiting for unsigned extrinsic in block with hash {ext_hash:?}"
		))?,
	};

	let (hash, receipts) = match extrinsic.wait_for_success().await {
		Ok(p) => {
			log::info!("Successfully executed unsigned extrinsic {ext_hash:?}");
			let mut receipts = p
				.find::<PostRequestHandledEvent>()
				.filter_map(|ev| ev.ok().map(|e| e.0.commitment.0.into()))
				.collect::<Vec<_>>();
			let temp_2 = p
				.find::<PostResponseHandledEvent>()
				.filter_map(|ev| ev.ok().map(|e| e.0.commitment.0.into()))
				.collect::<Vec<H256>>();
			receipts.extend(temp_2);
			(p.block_hash(), receipts)
		},
		Err(err) => Err(refine_subxt_error(err))
			.context(format!("Error executing unsigned extrinsic {ext_hash:?}"))?,
	};
	Ok(Some((hash, receipts)))
}

/// Dry run extrinsic
pub async fn system_dry_run_unsigned<T: subxt::Config, Tx: Payload>(
	client: &OnlineClient<T>,
	payload: Tx,
) -> Result<DryRunResult, anyhow::Error>
where
	T::Signature: From<MultiSignature> + Send + Sync,
{
	let ext = client.tx().create_unsigned(&payload)?;
	let result = ext.dry_run(None).await?;
	Ok(result)
}
