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

use alloc::string::ToString;
use anyhow::anyhow;
use beefy_primitives::mmr::BeefyNextAuthoritySet;
use codec::{Decode, Encode};
use core::str::FromStr;
use core::{
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    time::Duration,
};
use ibc::prelude::*;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use tendermint_proto::Protobuf;

use crate::proto::{BeefyAuthoritySet, ClientState as RawClientState};

use crate::error::Error;

use crate::client_def::BeefyClient;
use ibc::{
    core::{ics02_client::client_state::ClientType, ics24_host::identifier::ChainId},
    Height,
};

/// Protobuf type url for Beefy ClientState
pub const BEEFY_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.beefy.v1.ClientState";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum RelayChain {
    Polkadot = 0,
    Kusama = 1,
    Rococo = 2,
}

impl Default for RelayChain {
    fn default() -> Self {
        RelayChain::Rococo
    }
}

impl Display for RelayChain {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl RelayChain {
    /// Yields the Order as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Polkadot => "Polkadot",
            Self::Kusama => "Kusama",
            Self::Rococo => "Rococo",
        }
    }

    // Parses the Order out from a i32.
    pub fn from_i32(nr: i32) -> Result<Self, anyhow::Error> {
        match nr {
            0 => Ok(Self::Polkadot),
            1 => Ok(Self::Kusama),
            2 => Ok(Self::Rococo),
            id => Err(anyhow!("Unknown relay chain {id}")),
        }
    }
}

impl FromStr for RelayChain {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim_start_matches("order_") {
            "polkadot" => Ok(Self::Polkadot),
            "kusama" => Ok(Self::Kusama),
            "rococo" => Ok(Self::Rococo),
            _ => Err(anyhow!("Unknown relay chain {s}")),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default, Eq)]
pub struct ClientState {
    /// The chain id
    pub chain_id: ChainId,
    /// Relay chain
    pub relay_chain: RelayChain,
    /// Latest mmr root hash
    pub mmr_root_hash: H256,
    /// block number for the latest mmr_root_hash
    pub latest_beefy_height: u32,
    /// Block height when the client was frozen due to a misbehaviour
    pub frozen_height: Option<Height>,
    /// latest parachain height
    pub latest_para_height: u32,
    /// ParaId of associated parachain
    pub para_id: u32,
    /// authorities for the current round
    pub authority: BeefyNextAuthoritySet<H256>,
    /// authorities for the next round
    pub next_authority_set: BeefyNextAuthoritySet<H256>,
}

impl Protobuf<RawClientState> for ClientState {}

impl ClientState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        relay_chain: RelayChain,
        para_id: u32,
        latest_para_height: u32,
        mmr_root_hash: H256,
        latest_beefy_height: u32,
        authority_set: BeefyNextAuthoritySet<H256>,
        next_authority_set: BeefyNextAuthoritySet<H256>,
    ) -> Result<ClientState, Error> {
        if authority_set.id >= next_authority_set.id {
            return Err(Error::Custom(
                "ClientState next authority set id must be greater than current authority set id"
                    .to_string(),
            ));
        }
        let chain_id = ChainId::new(relay_chain.to_string(), para_id.into());

        Ok(Self {
            chain_id,
            mmr_root_hash,
            latest_beefy_height,
            frozen_height: None,
            authority: authority_set,
            next_authority_set,
            relay_chain,
            latest_para_height,
            para_id,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeOptions;

impl ClientState {
    pub fn latest_height(&self) -> Height {
        Height::new(self.para_id.into(), self.latest_para_height.into())
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id.clone()
    }

    pub fn client_type() -> ClientType {
        "11-beefy".to_string()
    }

    pub fn frozen_height(&self) -> Option<Height> {
        self.frozen_height
    }
}

impl ibc::core::ics02_client::client_state::ClientState for ClientState {
    type UpgradeOptions = UpgradeOptions;
    type ClientDef = BeefyClient;

    fn chain_id(&self) -> ChainId {
        self.chain_id()
    }

    fn client_def(&self) -> Self::ClientDef {
        BeefyClient::default()
    }

    fn client_type(&self) -> ClientType {
        Self::client_type()
    }

    fn latest_height(&self) -> Height {
        self.latest_height()
    }

    fn frozen_height(&self) -> Option<Height> {
        self.frozen_height()
    }

    fn upgrade(
        self,
        _upgrade_height: Height,
        _upgrade_options: UpgradeOptions,
        _chain_id: ChainId,
    ) -> Self {
        self
    }

    fn expired(&self, _elapsed: Duration) -> bool {
        false
    }

    fn encode_to_vec(&self) -> Result<Vec<u8>, tendermint_proto::Error> {
        self.encode_vec()
    }
}

impl TryFrom<RawClientState> for ClientState {
    type Error = Error;

    fn try_from(raw: RawClientState) -> Result<Self, Self::Error> {
        let authority_set = raw
            .authority
            .and_then(|set| {
                Some(BeefyNextAuthoritySet {
                    id: set.id,
                    len: set.len,
                    root: H256::decode(&mut &*set.authority_root).ok()?,
                })
            })
            .ok_or_else(|| Error::Custom(format!("Current authority set is missing")))?;

        let next_authority_set = raw
            .next_authority_set
            .and_then(|set| {
                Some(BeefyNextAuthoritySet {
                    id: set.id,
                    len: set.len,
                    root: H256::decode(&mut &*set.authority_root).ok()?,
                })
            })
            .ok_or_else(|| Error::Custom(format!("Next authority set is missing")))?;

        let mmr_root_hash = H256::decode(&mut &*raw.mmr_root_hash)?;
        let relay_chain = RelayChain::from_i32(raw.relay_chain)?;
        let chain_id = ChainId::new(relay_chain.to_string(), raw.para_id.into());

        Ok(Self {
            chain_id,
            mmr_root_hash,
            latest_beefy_height: raw.latest_beefy_height,
            frozen_height: raw
                .frozen_height
                .map(|height| Height::new(raw.para_id.into(), height)),
            authority: authority_set,
            next_authority_set,
            relay_chain,
            latest_para_height: raw.latest_para_height,
            para_id: raw.para_id,
        })
    }
}

impl From<ClientState> for RawClientState {
    fn from(client_state: ClientState) -> Self {
        RawClientState {
            mmr_root_hash: client_state.mmr_root_hash.encode(),
            latest_beefy_height: client_state.latest_beefy_height,
            frozen_height: client_state
                .frozen_height
                .map(|frozen_height| frozen_height.revision_height),
            authority: Some(BeefyAuthoritySet {
                id: client_state.authority.id,
                len: client_state.authority.len,
                authority_root: client_state.authority.root.encode(),
            }),
            next_authority_set: Some(BeefyAuthoritySet {
                id: client_state.next_authority_set.id,
                len: client_state.next_authority_set.len,
                authority_root: client_state.next_authority_set.root.encode(),
            }),
            relay_chain: client_state.relay_chain as i32,
            para_id: client_state.para_id,
            latest_para_height: client_state.latest_para_height,
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::mock::AnyClientState;

    pub fn get_dummy_beefy_state() -> AnyClientState {
        AnyClientState::Beefy(
            ClientState::new(
                RelayChain::Rococo,
                2000,
                0,
                Default::default(),
                0,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        )
    }
}
