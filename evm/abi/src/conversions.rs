// Copyright (C) 2022 Polytope Labs.
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

//! Convenient type conversions

use crate::{
    beefy::{
        AuthoritySetCommitment, BeefyConsensusProof, BeefyConsensusState, BeefyMmrLeaf, Commitment,
        IntermediateState, Node, Parachain, ParachainProof, Payload, RelayChainProof,
        SignedCommitment, Vote,
    },
    evm_host::EvmHostEvents,
    handler::StateMachineUpdatedFilter,
    shared_types::{PostRequest, PostResponse, StateMachineHeight},
};
use anyhow::anyhow;
use beefy_verifier_primitives::{
    BeefyNextAuthoritySet, ConsensusMessage, ConsensusState, MmrProof,
};
use ismp::{host::StateMachine, router};
use merkle_mountain_range::{leaf_index_to_mmr_size, leaf_index_to_pos, mmr_position_to_k_index};
use primitive_types::H256;
use std::str::FromStr;

impl From<beefy_verifier_primitives::ParachainProof> for ParachainProof {
    fn from(value: beefy_verifier_primitives::ParachainProof) -> Self {
        ParachainProof {
            parachain: value
                .parachains
                .into_iter()
                .map(|parachain| Parachain {
                    index: parachain.index.into(),
                    id: parachain.para_id.into(),
                    header: parachain.header.into(),
                })
                .collect::<Vec<_>>()[0]
                .clone(),
            proof: value
                .proof
                .into_iter()
                .map(|layer| {
                    layer
                        .into_iter()
                        .map(|(index, node)| Node { k_index: index.into(), node: node.into() })
                        .collect()
                })
                .collect(),
        }
    }
}

impl From<ConsensusMessage> for BeefyConsensusProof {
    fn from(message: ConsensusMessage) -> Self {
        BeefyConsensusProof { relay: message.mmr.into(), parachain: message.parachain.into() }
    }
}

impl From<MmrProof> for RelayChainProof {
    fn from(value: MmrProof) -> Self {
        let leaf_index = value.mmr_proof.leaf_indices[0];
        let k_index = mmr_position_to_k_index(
            vec![leaf_index_to_pos(leaf_index)],
            leaf_index_to_mmr_size(leaf_index),
        )[0]
        .1;

        RelayChainProof {
            signed_commitment: SignedCommitment {
                commitment: Commitment {
                    payload: vec![Payload {
                        id: b"mh".clone(),
                        data: value
                            .signed_commitment
                            .commitment
                            .payload
                            .get_raw(b"mh")
                            .unwrap()
                            .clone()
                            .into(),
                    }],
                    block_number: value.signed_commitment.commitment.block_number.into(),
                    validator_set_id: value.signed_commitment.commitment.validator_set_id.into(),
                },
                votes: value
                    .signed_commitment
                    .signatures
                    .into_iter()
                    .map(|a| Vote {
                        signature: a.signature.to_vec().into(),
                        authority_index: a.index.into(),
                    })
                    .collect(),
            },
            latest_mmr_leaf: BeefyMmrLeaf {
                version: 0.into(),
                parent_number: value.latest_mmr_leaf.parent_number_and_hash.0.into(),
                parent_hash: value.latest_mmr_leaf.parent_number_and_hash.1.into(),
                next_authority_set: value.latest_mmr_leaf.beefy_next_authority_set.into(),
                extra: value.latest_mmr_leaf.leaf_extra.into(),
                k_index: k_index.into(),
                leaf_index: leaf_index.into(),
            },
            mmr_proof: value.mmr_proof.items.into_iter().map(Into::into).collect(),
            proof: value
                .authority_proof
                .into_iter()
                .map(|layer| {
                    layer
                        .into_iter()
                        .map(|(index, node)| Node { k_index: index.into(), node: node.into() })
                        .collect()
                })
                .collect(),
        }
    }
}

impl From<BeefyNextAuthoritySet<H256>> for AuthoritySetCommitment {
    fn from(value: BeefyNextAuthoritySet<H256>) -> Self {
        AuthoritySetCommitment {
            id: value.id.into(),
            len: value.len.into(),
            root: value.keyset_commitment.into(),
        }
    }
}

impl From<ConsensusState> for BeefyConsensusState {
    fn from(value: ConsensusState) -> Self {
        BeefyConsensusState {
            latest_height: value.latest_beefy_height.into(),
            beefy_activation_block: value.beefy_activation_block.into(),
            current_authority_set: value.current_authorities.into(),
            next_authority_set: value.next_authorities.into(),
        }
    }
}

impl From<BeefyConsensusState> for ConsensusState {
    fn from(value: BeefyConsensusState) -> Self {
        ConsensusState {
            beefy_activation_block: value.beefy_activation_block.as_u32(),
            latest_beefy_height: value.latest_height.as_u32(),
            mmr_root_hash: Default::default(),
            current_authorities: BeefyNextAuthoritySet {
                id: value.current_authority_set.id.as_u64(),
                len: value.current_authority_set.len.as_u32(),
                keyset_commitment: value.current_authority_set.root.into(),
            },
            next_authorities: BeefyNextAuthoritySet {
                id: value.next_authority_set.id.as_u64(),
                len: value.next_authority_set.len.as_u32(),
                keyset_commitment: value.next_authority_set.root.into(),
            },
        }
    }
}

impl From<IntermediateState> for local::IntermediateState {
    fn from(value: IntermediateState) -> Self {
        local::IntermediateState {
            height: local::StateMachineHeight {
                state_machine_id: value.state_machine_id.as_u32(),
                height: value.height.as_u32(),
            },
            commitment: local::StateCommitment {
                timestamp: value.commitment.timestamp.as_u64(),
                state_root: H256(value.commitment.state_root),
                overlay_root: H256(value.commitment.overlay_root),
            },
        }
    }
}

impl From<router::PostResponse> for PostResponse {
    fn from(value: router::PostResponse) -> Self {
        PostResponse {
            request: value.post.into(),
            response: value.response.into(),
            timeout_timestamp: value.timeout_timestamp.into(),
            gaslimit: value.gas_limit.into(),
        }
    }
}

impl From<router::Post> for PostRequest {
    fn from(value: router::Post) -> Self {
        PostRequest {
            source: value.source.to_string().as_bytes().to_vec().into(),
            dest: value.dest.to_string().as_bytes().to_vec().into(),
            nonce: value.nonce.into(),
            from: value.from.into(),
            to: value.to.into(),
            timeout_timestamp: value.timeout_timestamp.into(),
            body: value.data.into(),
            gaslimit: value.gas_limit.into(),
        }
    }
}

impl TryFrom<ismp::consensus::StateMachineHeight> for StateMachineHeight {
    type Error = anyhow::Error;
    fn try_from(value: ismp::consensus::StateMachineHeight) -> Result<Self, anyhow::Error> {
        Ok(StateMachineHeight {
            state_machine_id: match value.id.state_id {
                StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
                state_machine => Err(anyhow!("Unsupported state machine {state_machine:?}"))?,
            },
            height: value.height.into(),
        })
    }
}

impl From<StateMachineUpdatedFilter> for ismp::events::StateMachineUpdated {
    fn from(value: StateMachineUpdatedFilter) -> Self {
        ismp::events::StateMachineUpdated {
            state_machine_id: ismp::consensus::StateMachineId {
                state_id: StateMachine::Kusama(value.state_machine_id.low_u64() as u32),
                consensus_state_id: Default::default(),
            },
            latest_height: value.height.low_u64(),
        }
    }
}

impl TryFrom<EvmHostEvents> for ismp::events::Event {
    type Error = anyhow::Error;
    fn try_from(event: EvmHostEvents) -> Result<Self, Self::Error> {
        match event {
            EvmHostEvents::GetRequestEventFilter(get) =>
                Ok(ismp::events::Event::GetRequest(router::Get {
                    source: StateMachine::from_str(&String::from_utf8(get.source.0.into())?)
                        .map_err(|e| anyhow!("{}", e))?,
                    dest: StateMachine::from_str(&String::from_utf8(get.dest.0.into())?)
                        .map_err(|e| anyhow!("{}", e))?,
                    nonce: get.nonce.low_u64(),
                    from: get.from.0.into(),
                    keys: get.keys.into_iter().map(|key| key.0.into()).collect(),
                    height: get.height.low_u64(),
                    timeout_timestamp: get.timeout_timestamp.low_u64(),
                    gas_limit: get.gaslimit.low_u64(),
                })),
            EvmHostEvents::PostRequestEventFilter(post) =>
                Ok(ismp::events::Event::PostRequest(router::Post {
                    source: StateMachine::from_str(&String::from_utf8(post.source.0.into())?)
                        .map_err(|e| anyhow!("{}", e))?,
                    dest: StateMachine::from_str(&String::from_utf8(post.dest.0.into())?)
                        .map_err(|e| anyhow!("{}", e))?,
                    nonce: post.nonce.low_u64(),
                    from: post.from.0.into(),
                    to: post.to.0.into(),
                    timeout_timestamp: post.timeout_timestamp.low_u64(),
                    data: post.data.0.into(),
                    gas_limit: post.gaslimit.low_u64(),
                })),
            EvmHostEvents::PostResponseEventFilter(resp) =>
                Ok(ismp::events::Event::PostResponse(router::PostResponse {
                    post: router::Post {
                        source: StateMachine::from_str(&String::from_utf8(resp.source.0.into())?)
                            .map_err(|e| anyhow!("{}", e))?,
                        dest: StateMachine::from_str(&String::from_utf8(resp.dest.0.into())?)
                            .map_err(|e| anyhow!("{}", e))?,
                        nonce: resp.nonce.low_u64(),
                        from: resp.from.0.into(),
                        to: resp.to.0.into(),
                        timeout_timestamp: resp.timeout_timestamp.low_u64(),
                        data: resp.data.0.into(),
                        gas_limit: resp.gaslimit.low_u64(),
                    },
                    response: resp.response.0.into(),
                    timeout_timestamp: resp.timeout_timestamp.low_u64(),
                    gas_limit: resp.res_gaslimit.low_u64(),
                })),
            event => Err(anyhow!("Unsupported event {event:?}")),
        }
    }
}

pub mod local {
    use super::H256;

    #[derive(Debug)]
    pub struct StateMachineHeight {
        pub state_machine_id: u32,
        pub height: u32,
    }

    #[derive(Debug)]
    pub struct StateCommitment {
        pub timestamp: u64,
        pub overlay_root: H256,
        pub state_root: H256,
    }

    #[derive(Debug)]
    pub struct IntermediateState {
        pub height: StateMachineHeight,
        pub commitment: StateCommitment,
    }
}
