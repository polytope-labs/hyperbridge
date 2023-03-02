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

use crate::{
    error::Error,
    proto::{
        BeaconBlockHeader as RawBeaconBlockHeader, ClientState as RawClientState,
        LightClientState as RawLightClientState, SyncCommittee as RawSyncCommittee,
    },
};
use alloc::{format, string::ToString, vec::Vec};
use anyhow::anyhow;
use core::{marker::PhantomData, time::Duration};
use ibc::{
    core::{
        ics02_client::{client_state::ClientType, error::Error as Ics02Error},
        ics24_host::identifier::ChainId,
    },
    Height,
};
use serde::{Deserialize, Serialize};

use crate::client_def::EthereumClient;
use ethereum_consensus::altair::mainnet::SYNC_COMMITTEE_SIZE;
use ethereum_consensus::bellatrix::{BeaconBlockHeader, SyncCommittee};
use ethereum_consensus::primitives::{BlsPublicKey, Root};
use ssz_rs::Vector;
use sync_committee_primitives::types::LightClientState;
use tendermint_proto::Protobuf;

/// Protobuf type url for Ethereum ClientState
pub const ETHEREUM_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.ethereum.v1.ClientState";

#[derive(PartialEq, Clone, Debug, Default, Eq)]
pub struct ClientState {
    pub state: LightClientState<SYNC_COMMITTEE_SIZE>,
    pub frozen_height: Option<u64>,
}

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
        let raw_light_client_state = raw
            .state
            .ok_or_else(|| Error::Ics02(Ics02Error::missing_raw_client_state()))?;
        let raw_beacon_header = raw_light_client_state
            .finalized_header
            .ok_or_else(|| Error::Ics02(Ics02Error::missing_raw_client_state()))?;
        let finalized_header = BeaconBlockHeader {
            slot: raw_beacon_header.slot,
            proposer_index: raw_beacon_header.proposer_index as usize,
            parent_root: Root::try_from(&raw_beacon_header.parent_root[..])
                .map_err(|_| Error::Custom("Invalid parent root".to_string()))?,
            state_root: Root::try_from(&raw_beacon_header.state_root[..])
                .map_err(|_| Error::Custom("Invalid state root".to_string()))?,
            body_root: Root::try_from(&raw_beacon_header.body_root[..])
                .map_err(|_| Error::Custom("Invalid body root".to_string()))?,
        };
        let raw_current_sync_committee = raw_light_client_state
            .current_sync_committee
            .ok_or_else(|| Error::Custom("Missing current sync committee".to_string()))?;
        let current_sync_committee = SyncCommittee {
            public_keys: Vector::<BlsPublicKey, SYNC_COMMITTEE_SIZE>::try_from(
                raw_current_sync_committee
                    .public_keys
                    .into_iter()
                    .map(|pub_key| BlsPublicKey::try_from(&pub_key[..]))
                    .collect::<Result<Vec<BlsPublicKey>, _>>()
                    .map_err(|_| Error::Custom("Invalid sync committee public keys".to_string()))?,
            )
            .map_err(|_| Error::Custom("Invalid sync committee public keys".to_string()))?,
            aggregate_public_key: BlsPublicKey::try_from(
                &raw_current_sync_committee.aggregate_public_key[..],
            )
            .map_err(|_| {
                Error::Custom("Invalid sync committee aggregate public keys".to_string())
            })?,
        };

        let raw_next_sync_committee = raw_light_client_state
            .next_sync_committee
            .ok_or_else(|| Error::Custom("Missing current sync committee".to_string()))?;
        let next_sync_committee = SyncCommittee {
            public_keys: Vector::<BlsPublicKey, SYNC_COMMITTEE_SIZE>::try_from(
                raw_next_sync_committee
                    .public_keys
                    .into_iter()
                    .map(|pub_key| BlsPublicKey::try_from(&pub_key[..]))
                    .collect::<Result<Vec<BlsPublicKey>, _>>()
                    .map_err(|_| Error::Custom("Invalid sync committee public keys".to_string()))?,
            )
            .map_err(|_| Error::Custom("Invalid sync committee public keys".to_string()))?,
            aggregate_public_key: BlsPublicKey::try_from(
                &raw_next_sync_committee.aggregate_public_key[..],
            )
            .map_err(|_| {
                Error::Custom("Invalid sync committee aggregate public keys".to_string())
            })?,
        };
        let light_client_state = LightClientState {
            finalized_header,
            latest_finalized_epoch: raw_light_client_state.latest_finalized_epoch,
            current_sync_committee,
            next_sync_committee,
        };

        Ok(ClientState {
            state: light_client_state,
            frozen_height: raw.frozen_height,
        })
    }
}

impl From<ClientState> for RawClientState {
    fn from(client_state: ClientState) -> Self {
        RawClientState {
            state: Some(RawLightClientState {
                finalized_header: Some(RawBeaconBlockHeader {
                    slot: client_state.state.finalized_header.slot,
                    proposer_index: client_state.state.finalized_header.proposer_index as u64,
                    parent_root: client_state
                        .state
                        .finalized_header
                        .parent_root
                        .as_bytes()
                        .to_vec(),
                    state_root: client_state
                        .state
                        .finalized_header
                        .state_root
                        .as_bytes()
                        .to_vec(),
                    body_root: client_state
                        .state
                        .finalized_header
                        .body_root
                        .as_bytes()
                        .to_vec(),
                }),
                latest_finalized_epoch: client_state.state.latest_finalized_epoch,
                current_sync_committee: Some(RawSyncCommittee {
                    public_keys: client_state
                        .state
                        .current_sync_committee
                        .public_keys
                        .iter()
                        .map(|pub_key| pub_key.as_slice().to_vec())
                        .collect(),
                    aggregate_public_key: client_state
                        .state
                        .current_sync_committee
                        .aggregate_public_key
                        .as_slice()
                        .to_vec(),
                }),
                next_sync_committee: Some(RawSyncCommittee {
                    public_keys: client_state
                        .state
                        .next_sync_committee
                        .public_keys
                        .iter()
                        .map(|pub_key| pub_key.as_slice().to_vec())
                        .collect(),
                    aggregate_public_key: client_state
                        .state
                        .next_sync_committee
                        .aggregate_public_key
                        .as_slice()
                        .to_vec(),
                }),
            }),
            frozen_height: client_state.frozen_height,
        }
    }
}
