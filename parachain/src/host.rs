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

use crate::{parachain, try_sending_with_tip, ParachainClient};
use codec::Encode;
use futures::Stream;
use ismp::{
    consensus::StateMachineId,
    messaging::{ConsensusMessage, Message},
};
use sp_core::{sr25519, Pair as _};
use std::pin::Pin;
use subxt::{
    config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
    ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
    tx::Signer,
};
use tesseract_primitives::{BoxStream, IsmpHost, StateMachineUpdated};

#[async_trait::async_trait]
impl<T> IsmpHost for ParachainClient<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
    <T::ExtrinsicParams as ExtrinsicParams<T::Index, T::Hash>>::OtherParams:
        Default + Send + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
    T::AccountId:
        From<sp_core::crypto::AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
    T::Signature: From<MultiSignature> + Send + Sync,
{
    fn name(&self) -> String {
        self.state_machine.to_string()
    }

    fn state_machine_id(&self) -> StateMachineId {
        StateMachineId {
            state_id: self.state_machine,
            consensus_client: ismp_parachain::consensus::PARACHAIN_CONSENSUS_ID,
        }
    }

    fn block_max_gas(&self) -> u64 {
        todo!()
    }

    async fn estimate_gas(&self, _msg: Vec<Message>) -> Result<u64, anyhow::Error> {
        todo!()
    }

    async fn consensus_notification<C>(
        &self,
        counterparty: C,
    ) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
    where
        C: IsmpHost + 'static,
    {
        self.consensus_notification_stream(counterparty).await
    }

    async fn state_machine_update_notification(
        &self,
        counterparty_id: StateMachineId,
    ) -> Pin<Box<dyn Stream<Item = Result<StateMachineUpdated, anyhow::Error>> + Send>> {
        self.state_machine_update_notification_stream(counterparty_id)
            .await
            .expect("Failed to get state machine notification stream")
    }

    async fn submit(&self, messages: Vec<Message>) -> Result<(), anyhow::Error> {
        let signer = InMemorySigner {
            account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
            signer: self.signer.clone(),
        };

        let tx =
            parachain::api::tx().ismp().handle(codec::Decode::decode(&mut &*messages.encode())?);
        let progress = try_sending_with_tip(&self.parachain, signer, tx).await?;
        let tx = progress.wait_for_in_block().await?;
        tx.wait_for_success().await?;

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
    fn account_id(&self) -> &T::AccountId {
        &self.account_id
    }

    fn address(&self) -> T::Address {
        self.account_id.clone().into()
    }

    fn sign(&self, payload: &[u8]) -> T::Signature {
        MultiSignature::Sr25519(self.signer.sign(&payload)).into()
    }
}
