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
#![allow(unused_imports)]
use crate::{
	beefy::IntermediateState,
	evm_host::EvmHostEvents,
	shared_types::{
		GetRequest, GetResponse, PostRequest, PostResponse, StateCommitment, StateMachineHeight,
		StorageValue,
	},
};

use anyhow::anyhow;
use ismp::{host::StateMachine, router};

use crate::evm_host::{GetRequestEventFilter, PostRequestEventFilter};
#[cfg(feature = "beefy")]
pub use beefy::*;
use ismp::{
	consensus::StateMachineId,
	events::{StateCommitmentVetoed, StateMachineUpdated, TimeoutHandled},
};
use primitive_types::{H160, H256};
use std::str::FromStr;

#[cfg(feature = "beefy")]
mod beefy {
	use crate::{
		beefy::{
			AuthoritySetCommitment, BeefyConsensusProof, BeefyConsensusState, BeefyMmrLeaf,
			Commitment, Node, Parachain, ParachainProof, Payload, RelayChainProof,
			SignedCommitment, Vote,
		},
		sp1_beefy::{MiniCommitment, ParachainHeader, PartialBeefyMmrLeaf},
	};
	use beefy_verifier_primitives::{ConsensusMessage, ConsensusState, MmrProof};
	use merkle_mountain_range::{leaf_index_to_mmr_size, leaf_index_to_pos};
	use polkadot_sdk::*;
	use primitive_types::H256;
	use sp_consensus_beefy::mmr::BeefyNextAuthoritySet;

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

	type SpCommitment = sp_consensus_beefy::Commitment<u32>;
	impl From<SpCommitment> for Commitment {
		fn from(value: SpCommitment) -> Self {
			Commitment {
				payload: vec![Payload {
					id: b"mh".clone(),
					data: value.payload.get_raw(b"mh").unwrap().clone().into(),
				}],
				block_number: value.block_number.into(),
				validator_set_id: value.validator_set_id.into(),
			}
		}
	}

	impl From<SpCommitment> for MiniCommitment {
		fn from(value: SpCommitment) -> Self {
			MiniCommitment {
				block_number: value.block_number.into(),
				validator_set_id: value.validator_set_id.into(),
			}
		}
	}

	type SpMmrLeaf = sp_consensus_beefy::mmr::MmrLeaf<u32, H256, H256, H256>;
	impl From<beefy_verifier_primitives::ParachainHeader> for ParachainHeader {
		fn from(value: beefy_verifier_primitives::ParachainHeader) -> Self {
			ParachainHeader { header: value.header.into(), id: value.para_id.into() }
		}
	}

	// useful for Sp1Beefy verifier
	impl From<SpMmrLeaf> for PartialBeefyMmrLeaf {
		fn from(value: SpMmrLeaf) -> Self {
			PartialBeefyMmrLeaf {
				version: 0.into(),
				parent_number: value.parent_number_and_hash.0.into(),
				parent_hash: value.parent_number_and_hash.1.into(),
				next_authority_set: value.beefy_next_authority_set.into(),
				extra: value.leaf_extra.into(),
			}
		}
	}

	impl From<MmrProof> for RelayChainProof {
		fn from(value: MmrProof) -> Self {
			let leaf_index = value.mmr_proof.leaf_indices[0];
			let k_index = mmr_primitives::mmr_position_to_k_index(
				vec![leaf_index_to_pos(leaf_index)],
				leaf_index_to_mmr_size(leaf_index),
			)[0]
			.1;

			RelayChainProof {
				signed_commitment: SignedCommitment {
					commitment: value.signed_commitment.commitment.into(),
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
		}
	}
}

impl From<router::PostRequest> for PostRequest {
	fn from(value: router::PostRequest) -> Self {
		PostRequest {
			source: value.source.to_string().as_bytes().to_vec().into(),
			dest: value.dest.to_string().as_bytes().to_vec().into(),
			nonce: value.nonce.into(),
			from: value.from.into(),
			to: value.to.into(),
			timeout_timestamp: value.timeout_timestamp.into(),
			body: value.body.into(),
		}
	}
}

impl TryFrom<PostRequest> for router::PostRequest {
	type Error = anyhow::Error;
	fn try_from(value: PostRequest) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(&String::from_utf8(value.source.to_vec())?)
				.map_err(|err| anyhow!("{err}"))?,
			dest: StateMachine::from_str(&String::from_utf8(value.dest.to_vec())?)
				.map_err(|err| anyhow!("{err}"))?,
			nonce: value.nonce,
			from: value.from.to_vec(),
			to: value.to.to_vec(),
			timeout_timestamp: value.timeout_timestamp.into(),
			body: value.body.to_vec(),
		})
	}
}

impl From<router::GetRequest> for GetRequest {
	fn from(value: router::GetRequest) -> Self {
		GetRequest {
			source: value.source.to_string().as_bytes().to_vec().into(),
			dest: value.dest.to_string().as_bytes().to_vec().into(),
			nonce: value.nonce,
			keys: value.keys.into_iter().map(Into::into).collect(),
			from: {
				let mut address = H160::default();
				address.0.copy_from_slice(&value.from);
				address.0.into()
			},
			context: value.context.into(),
			timeout_timestamp: value.timeout_timestamp,
			height: value.height,
		}
	}
}

impl From<router::GetResponse> for GetResponse {
	fn from(value: router::GetResponse) -> Self {
		GetResponse {
			request: value.get.into(),
			values: value
				.values
				.into_iter()
				.map(|storage_value| StorageValue {
					key: storage_value.key.into(),
					value: storage_value.value.unwrap_or_default().into(),
				})
				.collect(),
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

impl From<ismp::consensus::StateCommitment> for StateCommitment {
	fn from(value: ismp::consensus::StateCommitment) -> Self {
		StateCommitment {
			timestamp: value.timestamp.into(),
			state_root: value.state_root.0,
			overlay_root: value.overlay_root.unwrap_or_default().0,
		}
	}
}

impl TryFrom<EvmHostEvents> for ismp::events::Event {
	type Error = anyhow::Error;
	fn try_from(event: EvmHostEvents) -> Result<Self, Self::Error> {
		match event {
			EvmHostEvents::GetRequestEventFilter(get) =>
				Ok(ismp::events::Event::GetRequest(get.try_into()?)),
			EvmHostEvents::PostRequestEventFilter(post) =>
				Ok(ismp::events::Event::PostRequest(post.try_into()?)),
			EvmHostEvents::PostResponseEventFilter(resp) =>
				Ok(ismp::events::Event::PostResponse(router::PostResponse {
					post: router::PostRequest {
						source: StateMachine::from_str(&resp.dest).map_err(|e| anyhow!("{}", e))?,
						dest: StateMachine::from_str(&resp.source).map_err(|e| anyhow!("{}", e))?,
						nonce: resp.nonce.low_u64(),
						from: resp.to.0.into(),
						to: resp.from.0.into(),
						timeout_timestamp: resp.timeout_timestamp.low_u64(),
						body: resp.body.0.into(),
					},
					response: resp.response.0.into(),
					timeout_timestamp: resp.response_timeout_timestamp.low_u64(),
				})),
			EvmHostEvents::PostRequestHandledFilter(handled) =>
				Ok(ismp::events::Event::PostRequestHandled(ismp::events::RequestResponseHandled {
					commitment: handled.commitment.into(),
					relayer: handled.relayer.as_bytes().to_vec(),
				})),
			EvmHostEvents::GetRequestHandledFilter(handled) =>
				Ok(ismp::events::Event::GetRequestHandled(ismp::events::RequestResponseHandled {
					commitment: handled.commitment.into(),
					relayer: handled.relayer.as_bytes().to_vec(),
				})),

			EvmHostEvents::PostResponseHandledFilter(handled) =>
				Ok(ismp::events::Event::PostResponseHandled(ismp::events::RequestResponseHandled {
					commitment: handled.commitment.into(),
					relayer: handled.relayer.as_bytes().to_vec(),
				})),
			EvmHostEvents::StateMachineUpdatedFilter(filter) =>
				Ok(ismp::events::Event::StateMachineUpdated(StateMachineUpdated {
					state_machine_id: ismp::consensus::StateMachineId {
						state_id: StateMachine::from_str(&filter.state_machine_id)
							.map_err(|e| anyhow!("{}", e))?,
						consensus_state_id: Default::default(),
					},
					latest_height: filter.height.low_u64(),
				})),
			EvmHostEvents::PostRequestTimeoutHandledFilter(handled) => {
				let dest = StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(ismp::events::Event::PostRequestTimeoutHandled(TimeoutHandled {
					commitment: handled.commitment.into(),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::PostResponseTimeoutHandledFilter(handled) => {
				let dest = StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(ismp::events::Event::PostResponseTimeoutHandled(TimeoutHandled {
					commitment: handled.commitment.into(),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::GetRequestTimeoutHandledFilter(handled) => {
				let dest = StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(ismp::events::Event::GetRequestTimeoutHandled(TimeoutHandled {
					commitment: handled.commitment.into(),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::StateCommitmentVetoedFilter(vetoed) =>
				Ok(ismp::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
					height: ismp::consensus::StateMachineHeight {
						id: StateMachineId {
							state_id: StateMachine::from_str(&vetoed.state_machine_id)
								.map_err(|e| anyhow!("{}", e))?,
							consensus_state_id: Default::default(),
						},
						height: vetoed.height.low_u64(),
					},
					fisherman: vetoed.fisherman.as_bytes().to_vec(),
				})),
			EvmHostEvents::StateCommitmentReadFilter(_) |
			EvmHostEvents::HostFrozenFilter(_) |
			EvmHostEvents::HostWithdrawalFilter(_) |
			EvmHostEvents::HostParamsUpdatedFilter(_) |
			EvmHostEvents::PostResponseFundedFilter(_) |
			EvmHostEvents::RequestFundedFilter(_) => Err(anyhow!("Unsupported Event!"))?,
		}
	}
}

impl TryFrom<PostRequestEventFilter> for router::PostRequest {
	type Error = anyhow::Error;

	fn try_from(post: PostRequestEventFilter) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(&post.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&post.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: post.nonce.low_u64(),
			from: post.from.0.into(),
			to: post.to.0.into(),
			timeout_timestamp: post.timeout_timestamp.low_u64(),
			body: post.body.0.into(),
		})
	}
}

impl TryFrom<GetRequestEventFilter> for router::GetRequest {
	type Error = anyhow::Error;

	fn try_from(get: GetRequestEventFilter) -> Result<Self, Self::Error> {
		Ok(router::GetRequest {
			source: StateMachine::from_str(&get.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&get.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: get.nonce.low_u64(),
			from: get.from.0.into(),
			keys: get.keys.into_iter().map(|key| key.0.into()).collect(),
			height: get.height.low_u64(),
			context: get.context.as_ref().to_vec(),
			timeout_timestamp: get.timeout_timestamp.low_u64(),
		})
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
