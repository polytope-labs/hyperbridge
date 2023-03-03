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
        ancestry_proof::Message as RawAncestryProofMessage, client_message::Message as RawMessage,
        AncestorBlock as RawAncestorBlock, AncestryProof as RawAncestryProof,
        BeaconBlockHeader as RawBeaconBlockHeader, BlockRoots as RawBlockRoots,
        BlockRootsProof as RawBlockRootsProof, ClientMessage as RawClientMessage,
        ExecutionPayloadProof as RawExecutionPayloadProof, FinalityProof as RawFinalityProof,
        HistoricalRoots as RawHistoricalRoots, LightClientUpdate as RawLightClientUpdate,
        Misbehaviour as RawMisbehaviour, SyncAggregate as RawSyncAggregate,
        SyncCommittee as RawSyncCommittee, SyncCommitteeUpdate as RawSyncCommitteeUpdate,
    },
};
use alloc::vec::Vec;
use core::fmt::Debug;
use ethereum_consensus::bellatrix::mainnet::SYNC_COMMITTEE_SIZE;
use ethereum_consensus::bellatrix::{BeaconBlockHeader, SyncAggregate, SyncCommittee};
use ethereum_consensus::primitives::{BlsPublicKey, BlsSignature, Hash32, Root};
use ssz_rs::{Bitvector, Vector};
use sync_committee_primitives::types::{
    AncestorBlock, AncestryProof, BlockRootsProof, ExecutionPayloadProof, FinalityProof,
    SyncCommitteeUpdate,
};
use sync_committee_verifier::LightClientUpdate;
use tendermint_proto::Protobuf;

/// Protobuf type url for GRANDPA header
pub const ETHEREUM_CLIENT_MESSAGE_TYPE_URL: &str = "/ibc.lightclients.ethereum.v1.ClientMessage";

#[derive(Clone, Debug)]
pub struct Misbehaviour {
    pub header_1: LightClientUpdate,
    pub header_2: LightClientUpdate,
}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    /// This is the variant for header updates
    Header(LightClientUpdate),
    /// This is for submitting misbehaviors.
    Misbehaviour(Misbehaviour),
}

impl ibc::core::ics02_client::client_message::ClientMessage for ClientMessage {
    fn encode_to_vec(&self) -> Result<Vec<u8>, tendermint_proto::Error> {
        self.encode_vec()
    }
}

impl Protobuf<RawClientMessage> for ClientMessage {}

impl TryFrom<RawClientMessage> for ClientMessage {
    type Error = Error;

    fn try_from(raw_client_message: RawClientMessage) -> Result<Self, Self::Error> {
        let message = raw_client_message
            .message
            .ok_or_else(|| Error::Custom("Missing client message".to_string()))?;
        match message {
            RawMessage::Header(raw_light_client_update) => {
                let lc_update = from_raw_light_client_update(raw_light_client_update)?;
                Ok(ClientMessage::Header(lc_update))
            }
            RawMessage::Misbehaviour(raw_misbehaviour) => {
                Ok(ClientMessage::Misbehaviour(Misbehaviour {
                    header_1: from_raw_light_client_update(raw_misbehaviour.header_1.ok_or_else(
                        || Error::Custom("Missing light client update".to_string()),
                    )?)?,
                    header_2: from_raw_light_client_update(raw_misbehaviour.header_2.ok_or_else(
                        || Error::Custom("Missing light client update".to_string()),
                    )?)?,
                }))
            }
        }
    }
}

fn from_raw_light_client_update(
    raw_light_client_update: RawLightClientUpdate,
) -> Result<LightClientUpdate, Error> {
    let raw_attested_header = raw_light_client_update
        .attested_header
        .ok_or_else(|| Error::Custom("Missing attested header".to_string()))?;
    let attested_header = BeaconBlockHeader {
        slot: raw_attested_header.slot,
        proposer_index: raw_attested_header.proposer_index as usize,
        parent_root: Root::try_from(&raw_attested_header.parent_root[..])
            .map_err(|_| Error::Custom("Invalid parent root".to_string()))?,
        state_root: Root::try_from(&raw_attested_header.state_root[..])
            .map_err(|_| Error::Custom("Invalid state root".to_string()))?,
        body_root: Root::try_from(&raw_attested_header.body_root[..])
            .map_err(|_| Error::Custom("Invalid body root".to_string()))?,
    };

    let raw_finalized_header = raw_light_client_update
        .finalized_header
        .ok_or_else(|| Error::Custom("Missing finalized header".to_string()))?;
    let finalized_header = BeaconBlockHeader {
        slot: raw_finalized_header.slot,
        proposer_index: raw_finalized_header.proposer_index as usize,
        parent_root: Root::try_from(&raw_finalized_header.parent_root[..])
            .map_err(|_| Error::Custom("Invalid parent root".to_string()))?,
        state_root: Root::try_from(&raw_finalized_header.state_root[..])
            .map_err(|_| Error::Custom("Invalid state root".to_string()))?,
        body_root: Root::try_from(&raw_finalized_header.body_root[..])
            .map_err(|_| Error::Custom("Invalid body root".to_string()))?,
    };

    let raw_execution_payload = raw_light_client_update
        .execution_payload
        .ok_or_else(|| Error::Custom("Missing execution payload".to_string()))?;
    let execution_payload = ExecutionPayloadProof {
        state_root: Hash32::try_from(raw_execution_payload.state_root.as_slice())
            .map_err(|_| Error::Custom("invalid execution payload proof".to_string()))?,
        block_number: raw_execution_payload.block_number,
        multi_proof: raw_execution_payload
            .multi_proof
            .iter()
            .map(|node| {
                Hash32::try_from(node.as_slice())
                    .map_err(|_| Error::Custom("invalid execution payload proof".to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?,
        execution_payload_branch: raw_execution_payload
            .execution_payload_branch
            .iter()
            .map(|node| {
                Hash32::try_from(node.as_slice())
                    .map_err(|_| Error::Custom("invalid execution payload proof".to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    let raw_finality_proof = raw_light_client_update
        .finality_proof
        .ok_or_else(|| Error::Custom("Missing finality proof".to_string()))?;
    let finality_proof = FinalityProof {
        epoch: raw_finality_proof.epoch,
        finality_branch: raw_finality_proof
            .finality_branch
            .iter()
            .map(|node| {
                Hash32::try_from(node.as_slice())
                    .map_err(|_| Error::Custom("invalid finality proof".to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    let raw_sync_aggregate = raw_light_client_update
        .sync_aggregate
        .ok_or_else(|| Error::Custom("Missing sync aggregate".to_string()))?;
    let sync_aggregate = SyncAggregate {
        sync_committee_bits: {
            let bits = raw_sync_aggregate
                .sync_committee_bits
                .iter()
                .map(|val| *val == 1);
            Bitvector::<SYNC_COMMITTEE_SIZE>::from_iter(bits)
        },
        sync_committee_signature: BlsSignature::try_from(
            raw_sync_aggregate.sync_committee_signature.as_slice(),
        )
        .map_err(|_| Error::Custom("invalid sync aggregate".to_string()))?,
    };

    let sync_committee_update =
        if let Some(raw_sync_committee_update) = raw_light_client_update.sync_committee_update {
            let raw_next_sync_committee = raw_sync_committee_update
                .next_sync_committee
                .ok_or_else(|| Error::Custom("Missing next sync committee".to_string()))?;
            Some(SyncCommitteeUpdate {
                next_sync_committee: SyncCommittee {
                    public_keys: Vector::<BlsPublicKey, SYNC_COMMITTEE_SIZE>::try_from(
                        raw_next_sync_committee
                            .public_keys
                            .into_iter()
                            .map(|pub_key| BlsPublicKey::try_from(&pub_key[..]))
                            .collect::<Result<Vec<BlsPublicKey>, _>>()
                            .map_err(|_| {
                                Error::Custom("Invalid sync committee public keys".to_string())
                            })?,
                    )
                    .map_err(|_| Error::Custom("Invalid sync committee public keys".to_string()))?,
                    aggregate_public_key: BlsPublicKey::try_from(
                        &raw_next_sync_committee.aggregate_public_key[..],
                    )
                    .map_err(|_| {
                        Error::Custom("Invalid sync committee aggregate public keys".to_string())
                    })?,
                },
                next_sync_committee_branch: raw_sync_committee_update
                    .next_sync_committee_branch
                    .iter()
                    .map(|node| {
                        Hash32::try_from(node.as_slice())
                            .map_err(|_| Error::Custom("invalid sync committee update".to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        } else {
            None
        };

    let ancestor_blocks = raw_light_client_update
        .ancestor_blocks
        .into_iter()
        .map(|raw_ancestor_block| {
            let raw_header = raw_ancestor_block
                .header
                .ok_or_else(|| Error::Custom("Missing header in ancestor block".to_string()))?;
            let raw_execution_payload = raw_ancestor_block.execution_payload.ok_or_else(|| {
                Error::Custom("Missing execution payload proof in ancestor block".to_string())
            })?;
            let raw_ancestry_proof = raw_ancestor_block
                .ancestry_proof
                .ok_or_else(|| {
                    Error::Custom("Missing ancestry proof proof in ancestor block".to_string())
                })?
                .message
                .ok_or_else(|| Error::Custom("Empty ancestry proof".to_string()))?;
            let ancestry_proof = match raw_ancestry_proof {
                RawAncestryProofMessage::BlockRoots(raw_block_roots) => {
                    let raw_block_roots_proof = raw_block_roots
                        .block_roots_proof
                        .ok_or_else(|| Error::Custom("Empty block roots proof".to_string()))?;
                    AncestryProof::BlockRoots {
                        block_roots_proof: BlockRootsProof {
                            block_header_index: raw_block_roots_proof.block_header_index,
                            block_header_branch: raw_block_roots_proof
                                .block_header_branch
                                .iter()
                                .map(|node| {
                                    Hash32::try_from(node.as_slice()).map_err(|_| {
                                        Error::Custom("invalid block header branch".to_string())
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        },
                        block_roots_branch: raw_block_roots
                            .block_roots_branch
                            .iter()
                            .map(|node| {
                                Hash32::try_from(node.as_slice()).map_err(|_| {
                                    Error::Custom("invalid block header branch".to_string())
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    }
                }
                RawAncestryProofMessage::HistoricalRoots(raw_historical_roots) => {
                    let raw_block_roots_proof = raw_historical_roots
                        .block_roots_proof
                        .ok_or_else(|| Error::Custom("Empty block roots proof".to_string()))?;
                    AncestryProof::HistoricalRoots {
                        block_roots_proof: BlockRootsProof {
                            block_header_index: raw_block_roots_proof.block_header_index,
                            block_header_branch: raw_block_roots_proof
                                .block_header_branch
                                .iter()
                                .map(|node| {
                                    Hash32::try_from(node.as_slice()).map_err(|_| {
                                        Error::Custom("invalid block header branch".to_string())
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        },
                        historical_batch_proof: raw_historical_roots
                            .historical_batch_proof
                            .iter()
                            .map(|node| {
                                Hash32::try_from(node.as_slice()).map_err(|_| {
                                    Error::Custom("invalid historical proof".to_string())
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        historical_roots_proof: raw_historical_roots
                            .historical_roots_proof
                            .iter()
                            .map(|node| {
                                Hash32::try_from(node.as_slice()).map_err(|_| {
                                    Error::Custom("invalid historical proof".to_string())
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        historical_roots_index: raw_historical_roots.historical_roots_index,
                        historical_roots_branch: raw_historical_roots
                            .historical_roots_branch
                            .iter()
                            .map(|node| {
                                Hash32::try_from(node.as_slice()).map_err(|_| {
                                    Error::Custom("invalid historical proof".to_string())
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    }
                }
            };

            let ancestor_block = AncestorBlock {
                header: BeaconBlockHeader {
                    slot: raw_header.slot,
                    proposer_index: raw_header.proposer_index as usize,
                    parent_root: Root::try_from(&raw_header.parent_root[..])
                        .map_err(|_| Error::Custom("Invalid parent root".to_string()))?,
                    state_root: Root::try_from(&raw_header.state_root[..])
                        .map_err(|_| Error::Custom("Invalid state root".to_string()))?,
                    body_root: Root::try_from(&raw_header.body_root[..])
                        .map_err(|_| Error::Custom("Invalid body root".to_string()))?,
                },
                execution_payload: ExecutionPayloadProof {
                    state_root: Hash32::try_from(raw_execution_payload.state_root.as_slice())
                        .map_err(|_| {
                            Error::Custom("invalid execution payload proof".to_string())
                        })?,
                    block_number: raw_execution_payload.block_number,
                    multi_proof: raw_execution_payload
                        .multi_proof
                        .iter()
                        .map(|node| {
                            Hash32::try_from(node.as_slice()).map_err(|_| {
                                Error::Custom("invalid execution payload proof".to_string())
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    execution_payload_branch: raw_execution_payload
                        .execution_payload_branch
                        .iter()
                        .map(|node| {
                            Hash32::try_from(node.as_slice()).map_err(|_| {
                                Error::Custom("invalid execution payload proof".to_string())
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
                ancestry_proof,
            };
            Ok(ancestor_block)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let light_client_update = LightClientUpdate {
        attested_header,
        sync_committee_update,
        finalized_header,
        execution_payload,
        finality_proof,
        sync_aggregate,
        signature_slot: raw_light_client_update.signature_slot,
        ancestor_blocks,
    };

    Ok(light_client_update)
}

impl From<ClientMessage> for RawClientMessage {
    fn from(client_message: ClientMessage) -> Self {
        RawClientMessage {
            message: match client_message {
                ClientMessage::Header(lc_update) => {
                    Some(RawMessage::Header(to_raw_light_client_update(lc_update)))
                }
                ClientMessage::Misbehaviour(misbehaviour) => {
                    Some(RawMessage::Misbehaviour(RawMisbehaviour {
                        header_1: Some(to_raw_light_client_update(misbehaviour.header_1)),
                        header_2: Some(to_raw_light_client_update(misbehaviour.header_2)),
                    }))
                }
            },
        }
    }
}

fn to_raw_light_client_update(lc_update: LightClientUpdate) -> RawLightClientUpdate {
    RawLightClientUpdate {
        attested_header: Some(RawBeaconBlockHeader {
            slot: lc_update.attested_header.slot,
            proposer_index: lc_update.attested_header.proposer_index as u64,
            parent_root: lc_update.attested_header.parent_root.as_bytes().to_vec(),
            state_root: lc_update.attested_header.state_root.as_bytes().to_vec(),
            body_root: lc_update.attested_header.body_root.as_bytes().to_vec(),
        }),
        sync_committee_update: lc_update.sync_committee_update.map(|update| {
            RawSyncCommitteeUpdate {
                next_sync_committee: Some(RawSyncCommittee {
                    public_keys: update
                        .next_sync_committee
                        .public_keys
                        .iter()
                        .map(|key| key.as_slice().to_vec())
                        .collect(),
                    aggregate_public_key: update
                        .next_sync_committee
                        .aggregate_public_key
                        .as_slice()
                        .to_vec(),
                }),
                next_sync_committee_branch: update
                    .next_sync_committee_branch
                    .into_iter()
                    .map(|node| node.as_slice().to_vec())
                    .collect(),
            }
        }),
        finalized_header: Some(RawBeaconBlockHeader {
            slot: lc_update.finalized_header.slot,
            proposer_index: lc_update.finalized_header.proposer_index as u64,
            parent_root: lc_update.finalized_header.parent_root.as_bytes().to_vec(),
            state_root: lc_update.finalized_header.state_root.as_bytes().to_vec(),
            body_root: lc_update.finalized_header.body_root.as_bytes().to_vec(),
        }),
        execution_payload: Some(RawExecutionPayloadProof {
            state_root: lc_update.execution_payload.state_root.as_slice().to_vec(),
            block_number: lc_update.execution_payload.block_number,
            multi_proof: lc_update
                .execution_payload
                .multi_proof
                .iter()
                .map(|node| node.as_slice().to_vec())
                .collect(),
            execution_payload_branch: lc_update
                .execution_payload
                .execution_payload_branch
                .iter()
                .map(|node| node.as_slice().to_vec())
                .collect(),
        }),
        finality_proof: Some(RawFinalityProof {
            epoch: lc_update.finality_proof.epoch,
            finality_branch: lc_update
                .finality_proof
                .finality_branch
                .iter()
                .map(|node| node.as_slice().to_vec())
                .collect(),
        }),
        sync_aggregate: Some(RawSyncAggregate {
            sync_committee_bits: lc_update
                .sync_aggregate
                .sync_committee_bits
                .as_raw_slice()
                .to_vec(),
            sync_committee_signature: lc_update
                .sync_aggregate
                .sync_committee_signature
                .as_slice()
                .to_vec(),
        }),
        signature_slot: lc_update.signature_slot,
        ancestor_blocks: lc_update
            .ancestor_blocks
            .into_iter()
            .map(|ancestor_block| RawAncestorBlock {
                header: Some(RawBeaconBlockHeader {
                    slot: ancestor_block.header.slot,
                    proposer_index: ancestor_block.header.proposer_index as u64,
                    parent_root: ancestor_block.header.parent_root.as_bytes().to_vec(),
                    state_root: ancestor_block.header.state_root.as_bytes().to_vec(),
                    body_root: ancestor_block.header.body_root.as_bytes().to_vec(),
                }),
                execution_payload: Some(RawExecutionPayloadProof {
                    state_root: ancestor_block
                        .execution_payload
                        .state_root
                        .as_slice()
                        .to_vec(),
                    block_number: ancestor_block.execution_payload.block_number,
                    multi_proof: ancestor_block
                        .execution_payload
                        .multi_proof
                        .iter()
                        .map(|node| node.as_slice().to_vec())
                        .collect(),
                    execution_payload_branch: ancestor_block
                        .execution_payload
                        .execution_payload_branch
                        .iter()
                        .map(|node| node.as_slice().to_vec())
                        .collect(),
                }),
                ancestry_proof: match ancestor_block.ancestry_proof {
                    AncestryProof::BlockRoots {
                        block_roots_proof,
                        block_roots_branch,
                    } => Some(RawAncestryProof {
                        message: Some(RawAncestryProofMessage::BlockRoots(RawBlockRoots {
                            block_roots_proof: Some(RawBlockRootsProof {
                                block_header_index: block_roots_proof.block_header_index,
                                block_header_branch: block_roots_proof
                                    .block_header_branch
                                    .iter()
                                    .map(|node| node.as_slice().to_vec())
                                    .collect(),
                            }),
                            block_roots_branch: block_roots_branch
                                .iter()
                                .map(|node| node.as_slice().to_vec())
                                .collect(),
                        })),
                    }),
                    AncestryProof::HistoricalRoots {
                        block_roots_proof,
                        historical_batch_proof,
                        historical_roots_proof,
                        historical_roots_index,
                        historical_roots_branch,
                    } => Some(RawAncestryProof {
                        message: Some(RawAncestryProofMessage::HistoricalRoots(
                            RawHistoricalRoots {
                                block_roots_proof: Some(RawBlockRootsProof {
                                    block_header_index: block_roots_proof.block_header_index,
                                    block_header_branch: block_roots_proof
                                        .block_header_branch
                                        .iter()
                                        .map(|node| node.as_slice().to_vec())
                                        .collect(),
                                }),
                                historical_batch_proof: historical_batch_proof
                                    .iter()
                                    .map(|node| node.as_slice().to_vec())
                                    .collect(),
                                historical_roots_proof: historical_roots_proof
                                    .iter()
                                    .map(|node| node.as_slice().to_vec())
                                    .collect(),
                                historical_roots_index,
                                historical_roots_branch: historical_roots_branch
                                    .iter()
                                    .map(|node| node.as_slice().to_vec())
                                    .collect(),
                            },
                        )),
                    }),
                },
            })
            .collect(),
    }
}
