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

//! Testing utilities

use crate::{
    extrinsic::{send_extrinsic, Extrinsic, InMemorySigner},
    SubstrateClient,
};
use codec::Encode;
use futures::stream::StreamExt;
use hex_literal::hex;
use ismp_demo::{EvmParams, GetRequest, TransferParams};
use sp_core::Pair;
use std::time::Duration;
use subxt::{
    config::{
        extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
    },
    events::EventDetails,
    ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
};

impl<T, C> SubstrateClient<T, C>
where
    T: Send + Sync + Clone,
    C: subxt::Config + Send + Sync + Clone,
    C::Header: Send + Sync,
    <C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
        Default + Send + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
    C::AccountId: From<sp_core::crypto::AccountId32>
        + Into<C::Address>
        + Encode
        + Clone
        + 'static
        + Send
        + Sync,
    C::Signature: From<MultiSignature> + Send + Sync,
{
    pub async fn timestamp(&self) -> Result<Duration, anyhow::Error> {
        let addr: [u8; 32] =
            hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb");
        let timestamp = self.client.rpc().storage(&addr, None).await.unwrap().unwrap();
        let timestamp: u64 = codec::Decode::decode(&mut &*timestamp.0).unwrap();
        Ok(Duration::from_millis(timestamp))
    }

    pub fn latest_state_machine_height(&self) -> u64 {
        self.latest_state_machine_height.lock().clone()
    }

    pub async fn transfer(
        &self,
        params: TransferParams<C::AccountId, u128>,
    ) -> Result<(), anyhow::Error> {
        let signer = InMemorySigner {
            account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
            signer: self.signer.clone(),
        };

        let call = params.encode();
        let tx = Extrinsic::new("IsmpDemo", "transfer", call);

        let progress = send_extrinsic(&self.client, signer, tx).await?;
        let tx = progress.wait_for_in_block().await?;

        tx.wait_for_success().await?;

        Ok(())
    }

    pub async fn dispatch_to_evm(&self, params: EvmParams) -> Result<(), anyhow::Error> {
        let signer = InMemorySigner {
            account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
            signer: self.signer.clone(),
        };

        let call = params.encode();
        let tx = Extrinsic::new("IsmpDemo", "dispatch_to_evm", call);

        let progress = send_extrinsic(&self.client, signer, tx).await?;
        let tx = progress.wait_for_in_block().await?;

        tx.wait_for_success().await?;

        Ok(())
    }

    pub async fn get_request(&self, get_req: GetRequest) -> Result<(), anyhow::Error> {
        let signer = InMemorySigner {
            account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
            signer: self.signer.clone(),
        };

        let call = get_req.encode();
        let tx = Extrinsic::new("IsmpDemo", "get_request", call);

        let progress = send_extrinsic(&self.client, signer, tx).await?;
        let tx = progress.wait_for_in_block().await?;

        tx.wait_for_success().await?;

        Ok(())
    }

    pub async fn ismp_demo_events_stream(
        &self,
        count: usize,
        pallet_name: &'static str,
        variant_name: &'static str,
    ) -> Result<Vec<EventDetails<C>>, anyhow::Error> {
        let subscription = self.client.rpc().subscribe_best_block_headers().await?;
        let client = self.client.clone();
        let stream = subscription.filter_map(move |header| {
            let client = client.clone();
            async move {
                let events = client.events().at(header.ok()?.hash()).await.ok()?;

                let events = events
                    .iter()
                    .filter_map(|ev| {
                        let ev = ev.ok()?;
                        let ev_metadata = ev.event_metadata();
                        (ev_metadata.pallet.name() == pallet_name &&
                            ev_metadata.variant.name == variant_name)
                            .then(|| ev)
                    })
                    .collect();

                Some(events)
            }
        });

        let mut stream = Box::pin(stream);

        let mut total = 0;
        let mut values = vec![];
        while let Some(mut val) = stream.next().await {
            values.append(&mut val);
            total += values.len();
            if total >= count {
                return Ok(values)
            }
        }
        Err(anyhow::Error::msg("Stream ended"))
    }

    pub fn account(&self) -> C::AccountId {
        MultiSigner::Sr25519(self.signer.public()).into_account().into()
    }
}
