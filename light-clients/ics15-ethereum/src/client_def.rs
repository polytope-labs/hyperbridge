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

use crate::{client_state::ClientState, consensus_state::ConsensusState, error::Error};
use ibc::core::ics02_client::{
    client_consensus::ConsensusState as _, client_state::ClientState as _,
};

use crate::client_message::ClientMessage;
use alloc::{format, string::ToString, vec, vec::Vec};
use codec::Decode;
use core::marker::PhantomData;
use ibc::core::ics02_client::{
    client_def::{ClientDef, ConsensusUpdateResult},
    error::Error as Ics02Error,
};
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics04_channel::commitment::{AcknowledgementCommitment, PacketCommitment};
use ibc::core::ics04_channel::packet::Sequence;
use ibc::core::ics23_commitment::commitment::{
    CommitmentPrefix, CommitmentProofBytes, CommitmentRoot,
};
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use ibc::core::ics26_routing::context::ReaderContext;
use ibc::Height;

use tendermint_proto::Protobuf;

const CLIENT_STATE_UPGRADE_PATH: &[u8] = b"client-state-upgrade-path";
const CONSENSUS_STATE_UPGRADE_PATH: &[u8] = b"consensus-state-upgrade-path";

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EthereumClient;

impl ClientDef for EthereumClient {
    type ClientMessage = ClientMessage;
    type ClientState = ClientState;
    type ConsensusState = ConsensusState;
    fn verify_client_message<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        _client_state: Self::ClientState,
        _message: Self::ClientMessage,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn update_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        client_state: Self::ClientState,
        _message: Self::ClientMessage,
    ) -> Result<(Self::ClientState, ConsensusUpdateResult<Ctx>), Ics02Error> {
        unimplemented!()
    }

    fn update_state_on_misbehaviour(
        &self,
        client_state: Self::ClientState,
        _header: Self::ClientMessage,
    ) -> Result<Self::ClientState, Ics02Error> {
        unimplemented!()
    }

    fn check_for_misbehaviour<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        _client_state: Self::ClientState,
        _message: Self::ClientMessage,
    ) -> Result<bool, Ics02Error> {
        unimplemented!()
    }

    fn verify_upgrade_and_update_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        _old_client_state: &Self::ClientState,
        _upgrade_client_state: &Self::ClientState,
        _upgrade_consensus_state: &Self::ConsensusState,
        _proof_upgrade_client: Vec<u8>,
        _proof_upgrade_consensus_state: Vec<u8>,
    ) -> Result<(Self::ClientState, ConsensusUpdateResult<Ctx>), Ics02Error> {
        unimplemented!()
    }

    fn verify_client_consensus_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_state: &Self::ClientState,
        _height: Height,
        _prefix: &CommitmentPrefix,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _client_id: &ClientId,
        _consensus_height: Height,
        _expected_consensus_state: &Ctx::AnyConsensusState,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    // Consensus state will be verified in the verification functions  before these are called
    fn verify_connection_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: &ClientId,
        _client_state: &Self::ClientState,
        _height: Height,
        _prefix: &CommitmentPrefix,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _connection_id: &ConnectionId,
        _expected_connection_end: &ConnectionEnd,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn verify_channel_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: &ClientId,
        _client_state: &Self::ClientState,
        _height: Height,
        _prefix: &CommitmentPrefix,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _port_id: &PortId,
        _channel_id: &ChannelId,
        _expected_channel_end: &ChannelEnd,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn verify_client_full_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_state: &Self::ClientState,
        _height: Height,
        _prefix: &CommitmentPrefix,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _client_id: &ClientId,
        _expected_client_state: &Ctx::AnyClientState,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn verify_packet_data<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: &ClientId,
        _client_state: &Self::ClientState,
        _height: Height,
        _connection_end: &ConnectionEnd,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _port_id: &PortId,
        _channel_id: &ChannelId,
        _sequence: Sequence,
        _commitment: PacketCommitment,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn verify_packet_acknowledgement<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: &ClientId,
        _client_state: &Self::ClientState,
        _height: Height,
        _connection_end: &ConnectionEnd,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _port_id: &PortId,
        _channel_id: &ChannelId,
        _sequence: Sequence,
        _ack: AcknowledgementCommitment,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn verify_next_sequence_recv<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: &ClientId,
        _client_state: &Self::ClientState,
        _height: Height,
        _connection_end: &ConnectionEnd,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _port_id: &PortId,
        _channel_id: &ChannelId,
        _sequence: Sequence,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }

    fn verify_packet_receipt_absence<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: &ClientId,
        _client_state: &Self::ClientState,
        _height: Height,
        _connection_end: &ConnectionEnd,
        _proof: &CommitmentProofBytes,
        _root: &CommitmentRoot,
        _port_id: &PortId,
        _channel_id: &ChannelId,
        _sequence: Sequence,
    ) -> Result<(), Ics02Error> {
        unimplemented!()
    }
}
