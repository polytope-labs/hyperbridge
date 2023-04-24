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

//! tesseract-parachain client implementation

use codec::Encode;
use ismp::{host::StateMachine, messaging::CreateConsensusClient};
use sp_core::{bytes::from_hex, sr25519, Pair};
use subxt::{
    config::ExtrinsicParams,
    ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
    OnlineClient, PolkadotConfig,
};

mod byzantine;
mod codegen;
mod host;
mod notifications;
mod provider;

use crate::{
    host::InMemorySigner,
    parachain::api::{runtime_types, runtime_types::hyperbridge_runtime},
};
pub use codegen::*;

pub struct ParachainConfig {
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// RPC url for the relay chain. Unneeded if the host is a parachain.
    pub relay_chain: String,
    /// RPC url for the parachain
    pub parachain: String,
    /// Relayer account seed
    pub signer: String,
}

#[derive(Clone)]
pub struct ParachainClient<T: subxt::Config> {
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// Subxt client for the relay chain. Unneeded if the host is a parachain.
    relay_chain: OnlineClient<PolkadotConfig>,
    /// Subxt client for the parachain.
    parachain: OnlineClient<T>,
    /// RPC url for the parachain
    parachain_url: String,
    /// Private key of the signing account
    signer: sr25519::Pair,
}

impl<T> ParachainClient<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
    <T::ExtrinsicParams as ExtrinsicParams<T::Index, T::Hash>>::OtherParams: Default + Send,
    T::AccountId:
        From<sp_core::crypto::AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
    T::Signature: From<MultiSignature> + Send + Sync,
{
    pub async fn new(config: ParachainConfig) -> Result<Self, anyhow::Error> {
        let relay_chain = OnlineClient::from_url(&config.relay_chain).await?;
        let parachain = OnlineClient::<T>::from_url(&config.parachain).await?;

        let bytes = from_hex(&config.signer)?;
        let signer = sr25519::Pair::from_seed_slice(&bytes)?;

        Ok(ParachainClient {
            state_machine: config.state_machine,
            relay_chain,
            parachain,
            parachain_url: config.parachain,
            signer,
        })
    }

    pub async fn create_consensus_client(
        &self,
        message: CreateConsensusClient,
    ) -> Result<(), anyhow::Error> {
        let signer = InMemorySigner {
            account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
            signer: self.signer.clone(),
        };

        let tx = parachain::api::tx().sudo().sudo(hyperbridge_runtime::RuntimeCall::Ismp(
            runtime_types::pallet_ismp::pallet::Call::create_consensus_client {
                message: codec::Decode::decode(&mut &*message.encode())?,
            },
        ));
        let tx = self
            .parachain
            .tx()
            .sign_and_submit_then_watch_default(&tx, &signer)
            .await?
            .wait_for_in_block()
            .await?;

        tx.wait_for_success().await?;

        Ok(())
    }
}
