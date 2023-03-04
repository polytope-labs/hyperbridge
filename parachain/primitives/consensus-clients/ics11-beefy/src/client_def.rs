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

use core::fmt::Debug;

use crate::{
    client_message::ClientMessage, client_state::ClientState, consensus_state::ConsensusState,
    error::Error,
};
use ibc::{
    core::{
        ics02_client::{
            client_def::{ClientDef, ConsensusUpdateResult},
            error::Error as Ics02Error,
        },
        ics03_connection::connection::ConnectionEnd,
        ics04_channel::{
            channel::ChannelEnd,
            commitment::{AcknowledgementCommitment, PacketCommitment},
            packet::Sequence,
        },
        ics23_commitment::commitment::{CommitmentPrefix, CommitmentProofBytes, CommitmentRoot},
        ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId},
        ics26_routing::context::ReaderContext,
    },
    prelude::*,
    Height,
};

/// Dummy implementation of beefy client
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BeefyClient;

impl ClientDef for BeefyClient {
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
        Ok(())
    }

    fn update_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        client_state: Self::ClientState,
        _message: Self::ClientMessage,
    ) -> Result<(Self::ClientState, ConsensusUpdateResult<Ctx>), Ics02Error> {
        Ok((client_state, ConsensusUpdateResult::Batch(vec![])))
    }

    fn update_state_on_misbehaviour(
        &self,
        client_state: Self::ClientState,
        _header: Self::ClientMessage,
    ) -> Result<Self::ClientState, Ics02Error> {
        Ok(client_state)
    }

    fn check_for_misbehaviour<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        _client_state: Self::ClientState,
        _message: Self::ClientMessage,
    ) -> Result<bool, Ics02Error> {
        Ok(false)
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
        Err(Error::Custom("Beefy Client doesn't need client upgrades".to_string()).into())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
    }
}
