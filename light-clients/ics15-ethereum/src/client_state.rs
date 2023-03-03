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

use crate::{error::Error, proto::ClientState as RawClientState};
use alloc::{format, string::ToString, vec::Vec};
use anyhow::anyhow;
use core::{marker::PhantomData, time::Duration};
use ibc::{
    core::{ics02_client::client_state::ClientType, ics24_host::identifier::ChainId},
    Height,
};
use serde::{Deserialize, Serialize};

use crate::client_def::EthereumClient;
use tendermint_proto::Protobuf;

/// Protobuf type url for Ethereum ClientState
pub const ETHEREUM_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.ethereum.v1.ClientState";

#[derive(PartialEq, Clone, Debug, Default, Eq)]
pub struct ClientState;

impl Protobuf<RawClientState> for ClientState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeOptions;

impl ClientState {
    /// Verify that the client is at a sufficient height and unfrozen at the given height
    pub fn verify_height(&self, height: Height) -> Result<(), Error> {
        unimplemented!()
    }
}

impl ClientState {
    pub fn latest_height(&self) -> Height {
        unimplemented!()
    }

    pub fn chain_id(&self) -> ChainId {
        unimplemented!()
    }

    pub fn client_type() -> ClientType {
        "15-ethereum".to_string()
    }

    pub fn frozen_height(&self) -> Option<Height> {
        unimplemented!()
    }

    pub fn upgrade(
        mut self,
        _upgrade_height: Height,
        upgrade_options: UpgradeOptions,
        _chain_id: ChainId,
    ) -> Self {
        unimplemented!()
    }

    /// Check if the state is expired when `elapsed` time has passed since the latest consensus
    /// state timestamp
    pub fn expired(&self, elapsed: Duration) -> bool {
        unimplemented!()
    }

    pub fn with_frozen_height(self, h: Height) -> Result<Self, Error> {
        unimplemented!()
    }
}

impl ibc::core::ics02_client::client_state::ClientState for ClientState {
    type UpgradeOptions = UpgradeOptions;
    type ClientDef = EthereumClient;

    fn chain_id(&self) -> ChainId {
        unimplemented!()
    }

    fn client_def(&self) -> Self::ClientDef {
        unimplemented!()
    }

    fn client_type(&self) -> ClientType {
        unimplemented!()
    }

    fn latest_height(&self) -> Height {
        unimplemented!()
    }

    fn frozen_height(&self) -> Option<Height> {
        unimplemented!()
    }

    fn upgrade(
        self,
        upgrade_height: Height,
        upgrade_options: UpgradeOptions,
        chain_id: ChainId,
    ) -> Self {
        unimplemented!()
    }

    fn expired(&self, elapsed: Duration) -> bool {
        unimplemented!()
    }

    fn encode_to_vec(&self) -> Result<Vec<u8>, tendermint_proto::Error> {
        self.encode_vec()
    }
}

impl TryFrom<RawClientState> for ClientState {
    type Error = Error;

    fn try_from(raw: RawClientState) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}

impl From<ClientState> for RawClientState {
    fn from(client_state: ClientState) -> Self {
        unimplemented!()
    }
}
