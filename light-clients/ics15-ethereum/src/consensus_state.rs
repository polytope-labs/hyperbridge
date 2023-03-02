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

use alloc::{format, vec, vec::Vec};
use anyhow::anyhow;
use codec::Decode;
use core::{convert::Infallible, fmt::Debug};
use ethereum_consensus::configs::mainnet::SECONDS_PER_SLOT;
use ethereum_consensus::primitives::{Slot, GENESIS_SLOT};
use serde::Serialize;
use tendermint::time::Time;
use tendermint_proto::{google::protobuf as tpb, Protobuf};

use crate::proto::ConsensusState as RawConsensusState;

use crate::error::Error;
use ibc::{core::ics23_commitment::commitment::CommitmentRoot, timestamp::Timestamp, Height};
use sync_committee_verifier::LightClientUpdate;

/// Protobuf type url for GRANDPA Consensus State
pub const ETHEREUM_CONSENSUS_STATE_TYPE_URL: &str = "/ibc.lightclients.ethereum.v1.ConsensusState";

/// todo: Beacon chain genesis timestamp
pub const GENESIS_TIME: u64 = 0;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConsensusState {
    pub timestamp: Time,
    pub root: CommitmentRoot,
}

impl ConsensusState {
    pub fn new(root: Vec<u8>, timestamp: Time) -> Self {
        Self {
            timestamp,
            root: root.into(),
        }
    }

    pub fn from_header<H>(lc_update: LightClientUpdate) -> Result<(Height, Self), Error> {
        let root = CommitmentRoot::from_bytes(lc_update.execution_payload.state_root.as_slice());
        let timestamp = compute_timestamp_at_slot(lc_update.finalized_header.slot);
        Ok((
            Height::new(0, lc_update.finalized_header.slot),
            ConsensusState { timestamp, root },
        ))
    }
}

impl ibc::core::ics02_client::client_consensus::ConsensusState for ConsensusState {
    type Error = Infallible;

    fn root(&self) -> &CommitmentRoot {
        &self.root
    }

    fn timestamp(&self) -> Timestamp {
        self.timestamp.into()
    }

    fn encode_to_vec(&self) -> Result<Vec<u8>, tendermint_proto::Error> {
        self.encode_vec()
    }
}

impl Protobuf<RawConsensusState> for ConsensusState {}

impl TryFrom<RawConsensusState> for ConsensusState {
    type Error = Error;

    fn try_from(raw: RawConsensusState) -> Result<Self, Self::Error> {
        let prost_types::Timestamp { seconds, nanos } = raw
            .timestamp
            .ok_or_else(|| Error::Custom(format!("Invalid consensus state: missing timestamp")))?;
        let proto_timestamp = tpb::Timestamp { seconds, nanos };
        let timestamp = proto_timestamp.try_into().map_err(|e| {
            Error::Custom(format!("Invalid consensus state: invalid timestamp {e}"))
        })?;

        Ok(Self {
            root: raw.root.into(),
            timestamp,
        })
    }
}

impl From<ConsensusState> for RawConsensusState {
    fn from(value: ConsensusState) -> Self {
        let tpb::Timestamp { seconds, nanos } = value.timestamp.into();
        let timestamp = prost_types::Timestamp { seconds, nanos };

        RawConsensusState {
            timestamp: Some(timestamp),
            root: value.root.into_vec(),
        }
    }
}

fn compute_timestamp_at_slot(slot: Slot) -> Time {
    let slots_since_genesis = slot - GENESIS_SLOT;
    let timestamp_secs = GENESIS_TIME + (slots_since_genesis * SECONDS_PER_SLOT);
    let timestamp_nanos = timestamp_secs * 1_000_000_000;
    Timestamp::from_nanoseconds(timestamp_nanos)
        .expect("Valid timestamp")
        .into_tm_time()
        .unwrap()
}
