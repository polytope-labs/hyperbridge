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
use codec::Encode;
use sp_core::{sr25519, Pair};
use subxt::{
	config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	rpc::types::DryRunResult,
	tx::{Signer, TxPayload},
	Error, Metadata, OnlineClient,
};

/// Implements [`TxPayload`] for extrinsic encoding
pub struct Extrinsic {
	/// The pallet name, used to query the metadata
	pallet_name: String,
	/// The call name
	call_name: String,
	/// The encoded pallet call. Note that this should be the pallet call. Not runtime call
	encoded: Vec<u8>,
}

impl Extrinsic {
	/// Creates a new extrinsic ready to be sent with subxt.
	pub fn new(
		pallet_name: impl Into<String>,
		call_name: impl Into<String>,
		encoded_call: Vec<u8>,
	) -> Self {
		Extrinsic {
			pallet_name: pallet_name.into(),
			call_name: call_name.into(),
			encoded: encoded_call,
		}
	}
}

impl TxPayload for Extrinsic {
	fn encode_call_data_to(&self, metadata: &Metadata, out: &mut Vec<u8>) -> Result<(), Error> {
		// encode the pallet index
		let pallet = metadata.pallet_by_name_err(&self.pallet_name)?;
		let call_index = pallet
			.call_variant_by_name(&self.call_name)
			.ok_or_else(|| {
				Error::Other(format!(
					"Can't find {} in pallet {} metadata",
					self.call_name, self.pallet_name
				))
			})?
			.index;
		let pallet_index = pallet.index();
		pallet_index.encode_to(out);
		call_index.encode_to(out);

		// copy the encoded call to out
		out.extend_from_slice(&self.encoded);

		Ok(())
	}
}

#[derive(Clone)]
pub struct InMemorySigner<T: subxt::Config> {
	pub account_id: T::AccountId,
	pub signer: sr25519::Pair,
}

impl<T: subxt::Config> InMemorySigner<T>
where
	T::Signature: From<MultiSignature> + Send + Sync,
	T::AccountId:
		From<sp_core::crypto::AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
{
	pub fn new(pair: sr25519::Pair) -> Self {
		InMemorySigner {
			account_id: MultiSigner::Sr25519(pair.public()).into_account().into(),
			signer: pair,
		}
	}
}

impl<T: subxt::Config> Signer<T> for InMemorySigner<T>
where
	T::AccountId: Into<T::Address> + Clone + 'static,
	T::Signature: From<MultiSignature> + Send + Sync,
{
	fn account_id(&self) -> T::AccountId {
		self.account_id.clone()
	}

	fn address(&self) -> T::Address {
		self.account_id.clone().into()
	}

	fn sign(&self, payload: &[u8]) -> T::Signature {
		MultiSignature::Sr25519(self.signer.sign(&payload)).into()
	}
}

/// Send an unsigned extrinsic for ISMP messages.
pub async fn send_unsigned_extrinsic<T: subxt::Config, Tx: TxPayload>(
	client: &OnlineClient<T>,
	payload: Tx,
	wait_for_finalization: bool,
) -> Result<Option<T::Hash>, anyhow::Error>
where
	<T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
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

	let hash = match extrinsic.wait_for_success().await {
		Ok(p) => {
			log::info!("Successfully executed unsigned extrinsic {ext_hash:?}");
			p.block_hash()
		},
		Err(err) => Err(refine_subxt_error(err))
			.context(format!("Error executing unsigned extrinsic {ext_hash:?}"))?,
	};
	Ok(Some(hash))
}

/// Send a transaction
pub async fn send_extrinsic<T: subxt::Config, Tx: TxPayload>(
	client: &OnlineClient<T>,
	signer: InMemorySigner<T>,
	payload: Tx,
) -> Result<(), anyhow::Error>
where
	<T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
	T::Signature: From<MultiSignature> + Send + Sync,
{
	let other_params = BaseExtrinsicParamsBuilder::new();
	let ext = client.tx().create_signed(&payload, &signer, other_params.into()).await?;
	let progress = ext.submit_and_watch().await.context("Failed to submit signed extrinsic")?;
	let ext_hash = progress.extrinsic_hash();

	let extrinsic = match progress.wait_for_in_block().await {
		Ok(p) => p,
		Err(err) => Err(refine_subxt_error(err)).context(format!(
			"Error waiting for signed extrinsic in block with hash {ext_hash:?}"
		))?,
	};

	match extrinsic.wait_for_success().await {
		Ok(p) => p,
		Err(err) => Err(err).context(format!("Error executing signed extrinsic {ext_hash:?}"))?,
	};
	Ok(())
}

/// Dry run extrinsic
pub async fn system_dry_run_unsigned<T: subxt::Config, Tx: TxPayload>(
	client: &OnlineClient<T>,
	payload: Tx,
) -> Result<DryRunResult, anyhow::Error>
where
	<T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
	T::Signature: From<MultiSignature> + Send + Sync,
{
	let ext = client.tx().create_unsigned(&payload)?;
	let result = ext.dry_run(None).await?;
	Ok(result)
}

/// This prevents the runtime metadata from being displayed when module errors are encountered
fn refine_subxt_error(err: subxt::Error) -> anyhow::Error {
	match err {
		subxt::Error::Runtime(subxt::error::DispatchError::Module(ref err)) => {
			anyhow!(err.to_string())
		},
		_ => anyhow!(err),
	}
}
