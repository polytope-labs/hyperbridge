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
	beefy::Beefy::IntermediateState,
	evm_host::EvmHost::{
		GetRequest, GetRequestEvent, GetRequestHandled, GetRequestTimeoutHandled, GetResponse,
		HostFrozen, HostParamsUpdated, HostWithdrawal, PostRequest, PostRequestEvent,
		PostRequestHandled, PostRequestTimeoutHandled, PostResponse, PostResponseEvent,
		PostResponseFunded, PostResponseHandled, PostResponseTimeoutHandled, RequestFunded,
		StateCommitment, StateCommitmentRead, StateCommitmentVetoed as EvmStateCommitmentVetoed,
		StateMachineHeight, StateMachineUpdated as EvmStateMachineUpdated, StorageValue,
	},
};

use alloy_primitives::{FixedBytes, U256};
use anyhow::anyhow;
use ismp::{host::StateMachine, router};

#[cfg(feature = "beefy")]
pub use beefy::*;
use ismp::{
	consensus::StateMachineId,
	events::{StateCommitmentVetoed, StateMachineUpdated, TimeoutHandled},
};
use primitive_types::{H160, H256};
use std::str::FromStr;

/// Helper trait for converting primitive types to alloy U256
trait ToU256 {
	fn to_u256(self) -> U256;
}

impl ToU256 for u32 {
	fn to_u256(self) -> U256 {
		U256::from(self)
	}
}

impl ToU256 for u64 {
	fn to_u256(self) -> U256 {
		U256::from(self)
	}
}

impl ToU256 for usize {
	fn to_u256(self) -> U256 {
		U256::from(self)
	}
}

#[cfg(feature = "beefy")]
mod beefy {
	use super::ToU256;
	use crate::{
		beefy::Beefy::{
			AuthoritySetCommitment, BeefyConsensusProof, BeefyConsensusState, BeefyMmrLeaf,
			Commitment, Node, Parachain, ParachainProof, Payload, RelayChainProof,
			SignedCommitment, Vote,
		},
		sp1_beefy::SP1Beefy::{MiniCommitment, ParachainHeader, PartialBeefyMmrLeaf},
	};
	use alloy_primitives::{Bytes, FixedBytes, U256};
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
						index: parachain.index.to_u256(),
						id: parachain.para_id.to_u256(),
						header: Bytes::from(parachain.header),
					})
					.collect::<Vec<_>>()[0]
					.clone(),
				proof: value
					.proof
					.into_iter()
					.map(|layer| {
						layer
							.into_iter()
							.map(|(index, node)| Node {
								k_index: index.to_u256(),
								node: FixedBytes::from(node),
							})
							.collect()
					})
					.collect(),
			}
		}
	}

	impl From<ConsensusMessage> for BeefyConsensusProof {
		fn from(message: ConsensusMessage) -> Self {
			BeefyConsensusProof {
				relay: message.mmr.into(),
				parachain: message.parachain.into(),
			}
		}
	}

	type SpCommitment = sp_consensus_beefy::Commitment<u32>;
	impl From<SpCommitment> for Commitment {
		fn from(value: SpCommitment) -> Self {
			Commitment {
				payload: vec![Payload {
					id: FixedBytes::from(*b"mh"),
					data: Bytes::from(value.payload.get_raw(b"mh").unwrap().clone()),
				}],
				blockNumber: value.block_number.to_u256(),
				validatorSetId: value.validator_set_id.to_u256(),
			}
		}
	}

	impl From<SpCommitment> for MiniCommitment {
		fn from(value: SpCommitment) -> Self {
			MiniCommitment {
				blockNumber: value.block_number.to_u256(),
				validatorSetId: value.validator_set_id.to_u256(),
			}
		}
	}

	type SpMmrLeaf = sp_consensus_beefy::mmr::MmrLeaf<u32, H256, H256, H256>;
	impl From<beefy_verifier_primitives::ParachainHeader> for ParachainHeader {
		fn from(value: beefy_verifier_primitives::ParachainHeader) -> Self {
			ParachainHeader {
				header: Bytes::from(value.header),
				id: value.para_id.to_u256(),
			}
		}
	}

	// useful for Sp1Beefy verifier
	impl From<SpMmrLeaf> for PartialBeefyMmrLeaf {
		fn from(value: SpMmrLeaf) -> Self {
			use crate::sp1_beefy::SP1Beefy::AuthoritySetCommitment as Sp1AuthoritySetCommitment;
			PartialBeefyMmrLeaf {
				version: U256::ZERO,
				parentNumber: value.parent_number_and_hash.0.to_u256(),
				parentHash: FixedBytes::from(value.parent_number_and_hash.1 .0),
				nextAuthoritySet: Sp1AuthoritySetCommitment {
					id: value.beefy_next_authority_set.id.to_u256(),
					len: value.beefy_next_authority_set.len.to_u256(),
					root: FixedBytes::from(value.beefy_next_authority_set.keyset_commitment.0),
				},
				extra: FixedBytes::from(value.leaf_extra.0),
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
				signedCommitment: SignedCommitment {
					commitment: value.signed_commitment.commitment.into(),
					votes: value
						.signed_commitment
						.signatures
						.into_iter()
						.map(|a| Vote {
							signature: Bytes::from(a.signature.to_vec()),
							authorityIndex: a.index.to_u256(),
						})
						.collect(),
				},
				latestMmrLeaf: BeefyMmrLeaf {
					version: U256::ZERO,
					parentNumber: value.latest_mmr_leaf.parent_number_and_hash.0.to_u256(),
					parentHash: FixedBytes::from(value.latest_mmr_leaf.parent_number_and_hash.1 .0),
					nextAuthoritySet: value.latest_mmr_leaf.beefy_next_authority_set.into(),
					extra: FixedBytes::from(value.latest_mmr_leaf.leaf_extra.0),
					kIndex: k_index.to_u256(),
					leafIndex: leaf_index.to_u256(),
				},
				mmrProof: value
					.mmr_proof
					.items
					.into_iter()
					.map(|h| FixedBytes::from(h.0))
					.collect(),
				proof: value
					.authority_proof
					.into_iter()
					.map(|layer| {
						layer
							.into_iter()
							.map(|(index, node)| Node {
								k_index: index.to_u256(),
								node: FixedBytes::from(node),
							})
							.collect()
					})
					.collect(),
			}
		}
	}

	impl From<BeefyNextAuthoritySet<H256>> for AuthoritySetCommitment {
		fn from(value: BeefyNextAuthoritySet<H256>) -> Self {
			AuthoritySetCommitment {
				id: value.id.to_u256(),
				len: value.len.to_u256(),
				root: FixedBytes::from(value.keyset_commitment.0),
			}
		}
	}

	impl From<ConsensusState> for BeefyConsensusState {
		fn from(value: ConsensusState) -> Self {
			BeefyConsensusState {
				latestHeight: value.latest_beefy_height.to_u256(),
				beefyActivationBlock: value.beefy_activation_block.to_u256(),
				currentAuthoritySet: value.current_authorities.into(),
				nextAuthoritySet: value.next_authorities.into(),
			}
		}
	}

	impl From<BeefyConsensusState> for ConsensusState {
		fn from(value: BeefyConsensusState) -> Self {
			ConsensusState {
				beefy_activation_block: value.beefyActivationBlock.try_into().unwrap_or(0),
				latest_beefy_height: value.latestHeight.try_into().unwrap_or(0),
				mmr_root_hash: Default::default(),
				current_authorities: BeefyNextAuthoritySet {
					id: value.currentAuthoritySet.id.try_into().unwrap_or(0),
					len: value.currentAuthoritySet.len.try_into().unwrap_or(0),
					keyset_commitment: H256(value.currentAuthoritySet.root.0),
				},
				next_authorities: BeefyNextAuthoritySet {
					id: value.nextAuthoritySet.id.try_into().unwrap_or(0),
					len: value.nextAuthoritySet.len.try_into().unwrap_or(0),
					keyset_commitment: H256(value.nextAuthoritySet.root.0),
				},
			}
		}
	}
}

impl From<IntermediateState> for local::IntermediateState {
	fn from(value: IntermediateState) -> Self {
		local::IntermediateState {
			height: local::StateMachineHeight {
				state_machine_id: value.stateMachineId.try_into().unwrap_or(0),
				height: value.height.try_into().unwrap_or(0),
			},
			commitment: local::StateCommitment {
				timestamp: value.commitment.timestamp.try_into().unwrap_or(0),
				state_root: H256(value.commitment.stateRoot.0),
				overlay_root: H256(value.commitment.overlayRoot.0),
			},
		}
	}
}

impl From<router::PostResponse> for PostResponse {
	fn from(value: router::PostResponse) -> Self {
		PostResponse {
			request: value.post.into(),
			response: value.response.into(),
			timeoutTimestamp: value.timeout_timestamp,
		}
	}
}

impl From<router::PostRequest> for PostRequest {
	fn from(value: router::PostRequest) -> Self {
		PostRequest {
			source: value.source.to_string().into_bytes().into(),
			dest: value.dest.to_string().into_bytes().into(),
			nonce: value.nonce,
			from: value.from.into(),
			to: value.to.into(),
			timeoutTimestamp: value.timeout_timestamp,
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
			nonce: value.nonce.try_into().unwrap_or(0),
			from: value.from.to_vec(),
			to: value.to.to_vec(),
			timeout_timestamp: value.timeoutTimestamp.try_into().unwrap_or(0),
			body: value.body.to_vec(),
		})
	}
}

impl From<router::GetRequest> for GetRequest {
	fn from(value: router::GetRequest) -> Self {
		GetRequest {
			source: value.source.to_string().into_bytes().into(),
			dest: value.dest.to_string().into_bytes().into(),
			nonce: value.nonce,
			keys: value.keys.into_iter().map(Into::into).collect(),
			from: {
				let mut address = [0u8; 20];
				address.copy_from_slice(&value.from[..20.min(value.from.len())]);
				alloy_primitives::Address::from(address)
			},
			context: value.context.into(),
			timeoutTimestamp: value.timeout_timestamp,
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
			stateMachineId: match value.id.state_id {
				StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.to_u256(),
				state_machine => Err(anyhow!("Unsupported state machine {state_machine:?}"))?,
			},
			height: value.height.to_u256(),
		})
	}
}

impl From<ismp::consensus::StateCommitment> for StateCommitment {
	fn from(value: ismp::consensus::StateCommitment) -> Self {
		StateCommitment {
			timestamp: value.timestamp.to_u256(),
			stateRoot: FixedBytes::from(value.state_root.0),
			overlayRoot: FixedBytes::from(value.overlay_root.unwrap_or_default().0),
		}
	}
}

/// Enum representing all EvmHost events for conversion
pub enum EvmHostEvents {
	GetRequestEvent(GetRequestEvent),
	PostRequestEvent(PostRequestEvent),
	PostResponseEvent(PostResponseEvent),
	PostRequestHandled(PostRequestHandled),
	GetRequestHandled(GetRequestHandled),
	PostResponseHandled(PostResponseHandled),
	StateMachineUpdated(EvmStateMachineUpdated),
	PostRequestTimeoutHandled(PostRequestTimeoutHandled),
	PostResponseTimeoutHandled(PostResponseTimeoutHandled),
	GetRequestTimeoutHandled(GetRequestTimeoutHandled),
	StateCommitmentVetoed(EvmStateCommitmentVetoed),
	StateCommitmentRead(StateCommitmentRead),
	HostFrozen(HostFrozen),
	HostWithdrawal(HostWithdrawal),
	HostParamsUpdated(HostParamsUpdated),
	PostResponseFunded(PostResponseFunded),
	RequestFunded(RequestFunded),
}

impl TryFrom<EvmHostEvents> for ismp::events::Event {
	type Error = anyhow::Error;
	fn try_from(event: EvmHostEvents) -> Result<Self, Self::Error> {
		match event {
			EvmHostEvents::GetRequestEvent(get) =>
				Ok(ismp::events::Event::GetRequest(get.try_into()?)),
			EvmHostEvents::PostRequestEvent(post) =>
				Ok(ismp::events::Event::PostRequest(post.try_into()?)),
			EvmHostEvents::PostResponseEvent(resp) =>
				Ok(ismp::events::Event::PostResponse(router::PostResponse {
					post: router::PostRequest {
						source: StateMachine::from_str(&resp.dest).map_err(|e| anyhow!("{}", e))?,
						dest: StateMachine::from_str(&resp.source).map_err(|e| anyhow!("{}", e))?,
						nonce: resp.nonce.try_into().unwrap_or(0),
						from: resp.to.0.to_vec(),
						to: resp.from.0.to_vec(),
						timeout_timestamp: resp.timeoutTimestamp.try_into().unwrap_or(0),
						body: resp.body.to_vec(),
					},
					response: resp.response.to_vec(),
					timeout_timestamp: resp.responseTimeoutTimestamp.try_into().unwrap_or(0),
				})),
			EvmHostEvents::PostRequestHandled(handled) =>
				Ok(ismp::events::Event::PostRequestHandled(ismp::events::RequestResponseHandled {
					commitment: H256(handled.commitment.0),
					relayer: handled.relayer.0.to_vec(),
				})),
			EvmHostEvents::GetRequestHandled(handled) =>
				Ok(ismp::events::Event::GetRequestHandled(ismp::events::RequestResponseHandled {
					commitment: H256(handled.commitment.0),
					relayer: handled.relayer.0.to_vec(),
				})),

			EvmHostEvents::PostResponseHandled(handled) =>
				Ok(ismp::events::Event::PostResponseHandled(ismp::events::RequestResponseHandled {
					commitment: H256(handled.commitment.0),
					relayer: handled.relayer.0.to_vec(),
				})),
			EvmHostEvents::StateMachineUpdated(filter) =>
				Ok(ismp::events::Event::StateMachineUpdated(StateMachineUpdated {
					state_machine_id: ismp::consensus::StateMachineId {
						state_id: StateMachine::from_str(&filter.stateMachineId)
							.map_err(|e| anyhow!("{}", e))?,
						consensus_state_id: Default::default(),
					},
					latest_height: filter.height.try_into().unwrap_or(0),
				})),
			EvmHostEvents::PostRequestTimeoutHandled(handled) => {
				let dest = StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(ismp::events::Event::PostRequestTimeoutHandled(TimeoutHandled {
					commitment: H256(handled.commitment.0),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::PostResponseTimeoutHandled(handled) => {
				let dest = StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(ismp::events::Event::PostResponseTimeoutHandled(TimeoutHandled {
					commitment: H256(handled.commitment.0),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::GetRequestTimeoutHandled(handled) => {
				let dest = StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(ismp::events::Event::GetRequestTimeoutHandled(TimeoutHandled {
					commitment: H256(handled.commitment.0),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::StateCommitmentVetoed(vetoed) =>
				Ok(ismp::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
					height: ismp::consensus::StateMachineHeight {
						id: StateMachineId {
							state_id: StateMachine::from_str(&vetoed.stateMachineId)
								.map_err(|e| anyhow!("{}", e))?,
							consensus_state_id: Default::default(),
						},
						height: vetoed.height.try_into().unwrap_or(0),
					},
					fisherman: vetoed.fisherman.0.to_vec(),
				})),
			EvmHostEvents::StateCommitmentRead(_) |
			EvmHostEvents::HostFrozen(_) |
			EvmHostEvents::HostWithdrawal(_) |
			EvmHostEvents::HostParamsUpdated(_) |
			EvmHostEvents::PostResponseFunded(_) |
			EvmHostEvents::RequestFunded(_) => Err(anyhow!("Unsupported Event!"))?,
		}
	}
}

impl TryFrom<PostRequestEvent> for router::PostRequest {
	type Error = anyhow::Error;

	fn try_from(post: PostRequestEvent) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(&post.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&post.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: post.nonce.try_into().unwrap_or(0),
			from: post.from.0.to_vec(),
			to: post.to.0.to_vec(),
			timeout_timestamp: post.timeoutTimestamp.try_into().unwrap_or(0),
			body: post.body.to_vec(),
		})
	}
}

impl TryFrom<GetRequestEvent> for router::GetRequest {
	type Error = anyhow::Error;

	fn try_from(get: GetRequestEvent) -> Result<Self, Self::Error> {
		Ok(router::GetRequest {
			source: StateMachine::from_str(&get.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&get.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: get.nonce.try_into().unwrap_or(0),
			from: get.from.0.to_vec(),
			keys: get.keys.into_iter().map(|key| key.to_vec()).collect(),
			height: get.height.try_into().unwrap_or(0),
			context: get.context.to_vec(),
			timeout_timestamp: get.timeoutTimestamp.try_into().unwrap_or(0),
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
