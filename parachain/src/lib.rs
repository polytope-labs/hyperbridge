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

//! Parachain client implementation for tesseract.

use ismp::host::StateMachine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, sp_std::sync::Arc, sr25519, Pair};
use subxt::{
    config::{
        extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
    },
    ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
    tx::{TxPayload, TxProgress},
    OnlineClient, PolkadotConfig,
};

mod byzantine;
mod codegen;
mod config;
mod extrinsic;
mod host;
mod notifications;
mod provider;
#[cfg(feature = "testing")]
mod testing;

use crate::host::InMemorySigner;
pub use codegen::*;
pub use config::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParachainConfig {
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// RPC url for the relay chain. Unneeded if the host is a parachain.
    pub relay_chain: String,
    /// RPC url for the parachain
    pub parachain: String,
    /// Relayer account seed
    pub signer: String,
    /// Latest state machine height
    pub latest_state_machine_height: Option<u64>,
}

#[derive(Clone)]
pub struct ParachainClient<T: subxt::Config> {
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// Subxt client for the relay chain. Unneeded if the host is a parachain.
    relay_chain: OnlineClient<PolkadotConfig>,
    /// Subxt client for the parachain.
    parachain: OnlineClient<T>,
    /// Private key of the signing account
    pub signer: sr25519::Pair,
    /// Latest state machine height.
    latest_state_machine_height: Arc<Mutex<u64>>,
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
        // If latest height of the state machine on the counterparty is not provided in config
        // Set it to the latest parachain height
        let latest_state_machine_height =
            if let Some(latest_state_machine_height) = config.latest_state_machine_height {
                latest_state_machine_height
            } else {
                parachain
                    .rpc()
                    .header(None)
                    .await?
                    .expect("block header should be available")
                    .number()
                    .into()
            };
        Ok(ParachainClient {
            state_machine: config.state_machine,
            relay_chain,
            parachain,
            signer,
            latest_state_machine_height: Arc::new(Mutex::new(latest_state_machine_height)),
        })
    }

    pub fn account(&self) -> T::AccountId {
        MultiSigner::Sr25519(self.signer.public()).into_account().into()
    }
}

/// Send a transaction
pub async fn send_extrinsic<T: subxt::Config, Tx: TxPayload>(
    client: &OnlineClient<T>,
    signer: InMemorySigner<T>,
    payload: Tx,
) -> Result<TxProgress<T, OnlineClient<T>>, anyhow::Error>
where
    <T::ExtrinsicParams as ExtrinsicParams<T::Index, T::Hash>>::OtherParams:
        Default + Send + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
    T::Signature: From<MultiSignature> + Send + Sync,
{
    let other_params = BaseExtrinsicParamsBuilder::new();
    let progress =
        client.tx().sign_and_submit_then_watch(&payload, &signer, other_params.into()).await?;
    Ok(progress)
}
