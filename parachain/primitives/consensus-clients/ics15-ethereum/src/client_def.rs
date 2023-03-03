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

use crate::{client_state::ClientState, consensus_state::ConsensusState};
use ibc::core::ics02_client::client_consensus::ConsensusState as _;

use crate::client_message::ClientMessage;
use alloc::{string::ToString, vec::Vec};
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
use ssz_rs::Merkleized;

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
        client_state: Self::ClientState,
        message: Self::ClientMessage,
    ) -> Result<(), Ics02Error> {
        match message {
            ClientMessage::Header(light_client_update) => {
                sync_committee_verifier::verify_sync_committee_attestation(
                    client_state.state,
                    light_client_update,
                )
                .map_err(|e| Ics02Error::header_verification_failure(e.to_string()))?;
            }
            ClientMessage::Misbehaviour(misbehaviour) => {
                let slot_1 = misbehaviour.header_1.finalized_header.slot;
                let slot_2 = misbehaviour.header_2.finalized_header.slot;
                if slot_1 != slot_2 {
                    Err(Ics02Error::implementation_specific(
                        "Invalid Misbehaviiour :misbehaviour is from two different headers"
                            .to_string(),
                    ))?
                }
                let header_1_hash = misbehaviour
                    .header_1
                    .finalized_header
                    .clone()
                    .hash_tree_root()
                    .map_err(|_| {
                        Ics02Error::implementation_specific("Failed to hash header".to_string())
                    })?;
                let header_2_hash = misbehaviour
                    .header_1
                    .finalized_header
                    .clone()
                    .hash_tree_root()
                    .map_err(|_| {
                        Ics02Error::implementation_specific("Failed to hash header".to_string())
                    })?;
                if header_1_hash == header_2_hash {
                    Err(Ics02Error::implementation_specific(
                        "Invalid Misbehaviiour: The blocks are identical".to_string(),
                    ))?
                }

                sync_committee_verifier::verify_sync_committee_attestation(
                    client_state.state.clone(),
                    misbehaviour.header_1,
                )
                .map_err(|e| Ics02Error::header_verification_failure(e.to_string()))?;
                sync_committee_verifier::verify_sync_committee_attestation(
                    client_state.state,
                    misbehaviour.header_2,
                )
                .map_err(|e| Ics02Error::header_verification_failure(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn update_state<Ctx: ReaderContext>(
        &self,
        _ctx: &Ctx,
        _client_id: ClientId,
        client_state: Self::ClientState,
        message: Self::ClientMessage,
    ) -> Result<(Self::ClientState, ConsensusUpdateResult<Ctx>), Ics02Error> {
        match message {
            ClientMessage::Header(light_client_update) => {
                let (.., consensus_state) = ConsensusState::from_header(light_client_update)
                    .map_err(|e| Ics02Error::implementation_specific(e.to_string()))?;
                let cs = Ctx::AnyConsensusState::wrap(&consensus_state).ok_or_else(|| {
                    Ics02Error::unknown_consensus_state_type("Ctx::AnyConsensusState".to_string())
                })?;
                Ok((client_state, ConsensusUpdateResult::Single(cs)))
            }
            _ => unreachable!("02-client will check for Header before calling update_state; qed"),
        }
    }

    fn update_state_on_misbehaviour(
        &self,
        client_state: Self::ClientState,
        client_message: Self::ClientMessage,
    ) -> Result<Self::ClientState, Ics02Error> {
        let misbehaviour = match client_message {
            ClientMessage::Misbehaviour(misbehaviour) => misbehaviour,
            _ => unreachable!(
                "02-client will check for misbehaviour before calling update_state_on_misbehaviour; qed"
            ),
        };
        client_state
            .with_frozen_height(misbehaviour.header_1.finalized_header.slot)
            .map_err(|e| e.into())
    }

    fn check_for_misbehaviour<Ctx: ReaderContext>(
        &self,
        ctx: &Ctx,
        client_id: ClientId,
        _client_state: Self::ClientState,
        message: Self::ClientMessage,
    ) -> Result<bool, Ics02Error> {
        match message {
            ClientMessage::Misbehaviour(_) => Ok(true),
            ClientMessage::Header(lc_update) => {
                // Check if a consensus state is already installed; if so it should
                // match the untrusted header.
                let (height, header_consensus_state) = ConsensusState::from_header(lc_update)
                    .map_err(|e| Ics02Error::implementation_specific(e.to_string()))?;

                let existing_consensus_state =
                    match ctx.maybe_consensus_state(&client_id, height)? {
                        Some(cs) => {
                            let cs = cs.downcast::<ConsensusState>().ok_or(
                                Ics02Error::client_args_type_mismatch(
                                    ClientState::client_type().to_owned(),
                                ),
                            )?;
                            // If this consensus state matches, skip verification
                            // (optimization)
                            if header_consensus_state == cs {
                                // Header is already installed and matches the incoming
                                // header (already verified)
                                return Ok(false);
                            }
                            Some(cs)
                        }
                        None => None,
                    };

                // If the header has verified, but its corresponding consensus state
                // differs from the existing consensus state for that height, freeze the
                // client and return the installed consensus state.
                if let Some(cs) = existing_consensus_state {
                    if cs != header_consensus_state {
                        return Ok(true);
                    }
                }

                // todo: Are there any other checks needed
                Ok(false)
            }
        }
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
