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

use codec::Encode;
use sp_core::{sr25519, Pair};
use subxt::{
    config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
    ext::sp_runtime::MultiSignature,
    tx::{Signer, TxPayload, TxProgress},
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

pub struct InMemorySigner<T: subxt::Config> {
    pub account_id: T::AccountId,
    pub signer: sr25519::Pair,
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

/// Send a transaction
pub async fn send_extrinsic<T: subxt::Config, Tx: TxPayload>(
    client: &OnlineClient<T>,
    signer: InMemorySigner<T>,
    payload: Tx,
) -> Result<TxProgress<T, OnlineClient<T>>, anyhow::Error>
where
    <T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
        Default + Send + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
    T::Signature: From<MultiSignature> + Send + Sync,
{
    let other_params = BaseExtrinsicParamsBuilder::new();
    let progress =
        client.tx().sign_and_submit_then_watch(&payload, &signer, other_params.into()).await?;
    Ok(progress)
}
