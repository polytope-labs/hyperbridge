#![allow(clippy::all, ambiguous_glob_reexports)]

use anyhow::anyhow;
use beefy_verifier_primitives::{
    BeefyNextAuthoritySet, ConsensusMessage, ConsensusState, MmrProof,
};
use ismp::{
    consensus::StateMachineId,
    events::{Event, StateMachineUpdated},
    host::StateMachine,
    router,
};
use merkle_mountain_range_labs::{
    leaf_index_to_mmr_size, leaf_index_to_pos, mmr_position_to_k_index,
};
use primitive_types::H256;
use std::str::FromStr;

pub mod beefy;
pub mod i_ismp_host;
pub mod ismp_handler;
pub mod mock_module;
pub mod shared_types;

pub use beefy::*;
pub use i_ismp_host::*;
pub use ismp_handler::*;
pub use mock_module::*;
pub use shared_types::*;

impl TryFrom<IIsmpHostEvents> for Event {
    type Error = anyhow::Error;

    fn try_from(event: IIsmpHostEvents) -> Result<Event, anyhow::Error> {
        match event {
            IIsmpHostEvents::GetRequestEventFilter(get) => Ok(Event::GetRequest(router::Get {
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
            IIsmpHostEvents::PostRequestEventFilter(post) => Ok(Event::PostRequest(router::Post {
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
            IIsmpHostEvents::PostResponseEventFilter(resp) => {
                Ok(Event::PostResponse(router::PostResponse {
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
                }))
            }
        }
    }
}

impl From<StateMachineUpdatedFilter> for StateMachineUpdated {
    fn from(event: StateMachineUpdatedFilter) -> StateMachineUpdated {
        StateMachineUpdated {
            state_machine_id: StateMachineId {
                state_id: StateMachine::Kusama(event.state_machine_id.low_u64() as u32),
                consensus_state_id: Default::default(),
            },
            latest_height: event.height.low_u64(),
        }
    }
}

impl From<ConsensusMessage> for BeefyConsensusProof {
    fn from(message: ConsensusMessage) -> Self {
        BeefyConsensusProof {
            relay: message.mmr.into(),
            parachain: ParachainProof {
                parachain: message
                    .parachain
                    .parachains
                    .into_iter()
                    .map(|parachain| Parachain {
                        index: parachain.index.into(),
                        id: parachain.para_id.into(),
                        header: parachain.header.into(),
                    })
                    .collect::<Vec<_>>()[0]
                    .clone(),
                proof: message
                    .parachain
                    .proof
                    .into_iter()
                    .map(|layer| {
                        layer
                            .into_iter()
                            .map(|(index, node)| Node { k_index: index.into(), node: node.into() })
                            .collect()
                    })
                    .collect(),
            },
        }
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
            root: value.root.into(),
        }
    }
}

impl From<ConsensusState> for BeefyConsensusState {
    fn from(value: ConsensusState) -> Self {
        BeefyConsensusState {
            latest_height: value.latest_beefy_height.into(),
            beefy_activation_block: Default::default(),
            current_authority_set: value.current_authorities.into(),
            next_authority_set: value.next_authorities.into(),
        }
    }
}

impl From<BeefyConsensusState> for ConsensusState {
    fn from(value: BeefyConsensusState) -> Self {
        ConsensusState {
            latest_beefy_height: value.latest_height.as_u32(),
            mmr_root_hash: Default::default(),
            current_authorities: BeefyNextAuthoritySet {
                id: value.current_authority_set.id.as_u64(),
                len: value.current_authority_set.len.as_u32(),
                root: value.current_authority_set.root.into(),
            },
            next_authorities: BeefyNextAuthoritySet {
                id: value.next_authority_set.id.as_u64(),
                len: value.next_authority_set.len.as_u32(),
                root: value.next_authority_set.root.into(),
            },
        }
    }
}
