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
	beefy::Beefy::IntermediateState,
	evm_host::EvmHost::{
		GetRequest, GetRequestEvent, GetRequestHandled, GetRequestTimeoutHandled, GetResponse,
		HostFrozen, HostParamsUpdated, HostWithdrawal, PostRequest, PostRequestEvent,
		PostRequestHandled, PostRequestTimeoutHandled, PostResponse, PostResponseEvent,
		PostResponseFunded, PostResponseHandled, PostResponseTimeoutHandled, RequestFunded,
		StateCommitment, StateCommitmentRead, StateCommitmentVetoed as EvmStateCommitmentVetoed,
		StateMachineHeight, StateMachineUpdated as EvmStateMachineUpdated,
	},
};

use alloc::string::{String, ToString};
use alloy_primitives::{FixedBytes, U256};
use anyhow::anyhow;
use core::str::FromStr;
use ismp::{
	consensus::StateMachineId,
	events::{StateCommitmentVetoed, StateMachineUpdated, TimeoutHandled},
	host::StateMachine,
	router,
};
use primitive_types::{H160, H256};

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

mod beefy {
	use super::ToU256;
	use crate::{
		beefy::Beefy::{
			AuthoritySetCommitment, BeefyConsensusProof, BeefyConsensusState, BeefyMmrLeaf,
			Commitment, Parachain, ParachainProof, Payload, RelayChainProof, SignedCommitment,
			Vote,
		},
		sp1_beefy::SP1Beefy::{MiniCommitment, ParachainHeader, PartialBeefyMmrLeaf},
	};
	use alloc::vec;
	use alloc::vec::Vec;
	use alloy_primitives::{Bytes, FixedBytes, U256};
	use beefy_verifier_primitives::{
		ConsensusMessage, ConsensusState, MmrProof, ParachainHeader as BvpParachainHeader,
		ParachainProof as BvpParachainProof, SignatureWithAuthorityIndex,
		SignedCommitment as BvpSignedCommitment, Sp1BeefyProof, TSignature,
	};
	use polkadot_sdk::*;
	use primitive_types::H256;
	use sp_consensus_beefy::{
		Payload as BeefyPayload,
		mmr::{BeefyNextAuthoritySet, MmrLeafVersion},
	};
	use sp_mmr_primitives::LeafProof;

	impl From<beefy_verifier_primitives::ParachainProof> for ParachainProof {
		fn from(value: beefy_verifier_primitives::ParachainProof) -> Self {
			ParachainProof {
				parachains: value
					.parachains
					.into_iter()
					.map(|parachain| Parachain {
						index: parachain.index.to_u256(),
						id: parachain.para_id.to_u256(),
						header: Bytes::from(parachain.header),
					})
					.collect(),
				proof: value.proof.into_iter().map(|hash| FixedBytes::from(hash)).collect(),
				leafCount: value.total_leaves.to_u256(),
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
					id: FixedBytes::from(*b"mh"),
					data: Bytes::from(
						value.payload.get_raw(b"mh").expect("mmr payload not present").clone(),
					),
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
			ParachainHeader { header: Bytes::from(value.header), id: value.para_id.to_u256() }
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
					.map(|hash| FixedBytes::from(hash))
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
				beefy_activation_block: value
					.beefyActivationBlock
					.try_into()
					.expect("beefy activation block out of bounds"),
				latest_beefy_height: value
					.latestHeight
					.try_into()
					.expect("Beefy latest height out of bounds"),
				mmr_root_hash: Default::default(),
				current_authorities: BeefyNextAuthoritySet {
					id: value
						.currentAuthoritySet
						.id
						.try_into()
						.expect("current authority set id out of bounds"),
					len: value
						.currentAuthoritySet
						.len
						.try_into()
						.expect("current authority set length out of bounds"),
					keyset_commitment: H256(value.currentAuthoritySet.root.0),
				},
				next_authorities: BeefyNextAuthoritySet {
					id: value
						.nextAuthoritySet
						.id
						.try_into()
						.expect("next authority set out of bounds"),
					len: value
						.nextAuthoritySet
						.len
						.try_into()
						.expect("next authority set length out of bounds"),
					keyset_commitment: H256(value.nextAuthoritySet.root.0),
				},
			}
		}
	}

	impl From<PartialBeefyMmrLeaf> for sp_consensus_beefy::mmr::MmrLeaf<u32, H256, H256, H256> {
		fn from(value: PartialBeefyMmrLeaf) -> Self {
			let version: u8 = value.version.try_into().expect("mmr leaf version out of bounds");
			sp_consensus_beefy::mmr::MmrLeaf {
				version: MmrLeafVersion::new(version >> 5, version & 0b11111),
				parent_number_and_hash: (
					value.parentNumber.try_into().expect("parent number out of bounds"),
					H256(value.parentHash.0),
				),
				beefy_next_authority_set: BeefyNextAuthoritySet {
					id: value
						.nextAuthoritySet
						.id
						.try_into()
						.expect("next authority set id out of bounds"),
					len: value
						.nextAuthoritySet
						.len
						.try_into()
						.expect("next authority set len out of bounds"),
					keyset_commitment: H256(value.nextAuthoritySet.root.0),
				},
				leaf_extra: H256(value.extra.0),
			}
		}
	}

	impl From<ParachainHeader> for beefy_verifier_primitives::ParachainHeader {
		fn from(value: ParachainHeader) -> Self {
			beefy_verifier_primitives::ParachainHeader {
				header: value.header.to_vec(),
				para_id: value.id.try_into().expect("para id out of bounds"),
				// SP1 proves inclusion directly so any value here is fine.
				index: 0,
			}
		}
	}

	impl From<Parachain> for BvpParachainHeader {
		fn from(value: Parachain) -> Self {
			BvpParachainHeader {
				header: value.header.to_vec(),
				index: value.index.try_into().expect("parachain leaf index out of bounds"),
				para_id: value.id.try_into().expect("para id out of bounds"),
			}
		}
	}

	impl From<ParachainProof> for BvpParachainProof {
		fn from(value: ParachainProof) -> Self {
			BvpParachainProof {
				parachains: value.parachains.into_iter().map(Into::into).collect(),
				proof: value.proof.into_iter().map(|h| h.0).collect(),
				total_leaves: value.leafCount.try_into().expect("leaf count out of bounds"),
			}
		}
	}

	impl From<BeefyMmrLeaf> for SpMmrLeaf {
		fn from(value: BeefyMmrLeaf) -> Self {
			let version: u8 = value.version.try_into().expect("mmr leaf version out of bounds");
			sp_consensus_beefy::mmr::MmrLeaf {
				version: MmrLeafVersion::new(version >> 5, version & 0b11111),
				parent_number_and_hash: (
					value.parentNumber.try_into().expect("parent number out of bounds"),
					H256(value.parentHash.0),
				),
				beefy_next_authority_set: BeefyNextAuthoritySet {
					id: value
						.nextAuthoritySet
						.id
						.try_into()
						.expect("next authority set id out of bounds"),
					len: value
						.nextAuthoritySet
						.len
						.try_into()
						.expect("next authority set len out of bounds"),
					keyset_commitment: H256(value.nextAuthoritySet.root.0),
				},
				leaf_extra: H256(value.extra.0),
			}
		}
	}

	impl From<Commitment> for SpCommitment {
		/// BEEFY commitment reconstruction. Reassembles the `Payload` from its
		/// `(id, data)` entries, starting with the first entry and pushing the rest via
		/// `push_raw` (which re-sorts by id to keep the invariant `Payload` expects).
		fn from(value: Commitment) -> Self {
			let mut iter = value.payload.into_iter();
			let first = iter.next().expect("commitment has at least one payload entry");
			let mut payload = BeefyPayload::from_single_entry(first.id.0, first.data.to_vec());
			for p in iter {
				payload = payload.push_raw(p.id.0, p.data.to_vec());
			}
			sp_consensus_beefy::Commitment {
				payload,
				block_number: value.blockNumber.try_into().expect("block number out of bounds"),
				validator_set_id: value
					.validatorSetId
					.try_into()
					.expect("validator set id out of bounds"),
			}
		}
	}

	impl From<Vote> for SignatureWithAuthorityIndex {
		fn from(value: Vote) -> Self {
			let sig_bytes = value.signature.to_vec();
			let mut signature: TSignature = [0u8; 65];
			signature.copy_from_slice(&sig_bytes);
			SignatureWithAuthorityIndex {
				signature,
				index: value.authorityIndex.try_into().expect("authority index out of bounds"),
			}
		}
	}

	impl From<RelayChainProof> for MmrProof {
		fn from(value: RelayChainProof) -> Self {
			let leaf_index: u64 = value
				.latestMmrLeaf
				.leafIndex
				.try_into()
				.expect("mmr leaf index out of bounds");
			let items: Vec<H256> = value.mmrProof.into_iter().map(|h| H256(h.0)).collect();
			let mmr_proof = LeafProof {
				leaf_indices: vec![leaf_index],
				leaf_count: leaf_index.saturating_add(1),
				items,
			};

			MmrProof {
				signed_commitment: BvpSignedCommitment {
					commitment: value.signedCommitment.commitment.into(),
					signatures: value
						.signedCommitment
						.votes
						.into_iter()
						.map(Into::into)
						.collect(),
				},
				latest_mmr_leaf: value.latestMmrLeaf.into(),
				mmr_proof,
				authority_proof: value.proof.into_iter().map(|h| h.0).collect(),
			}
		}
	}

	impl From<BeefyConsensusProof> for ConsensusMessage {
		fn from(value: BeefyConsensusProof) -> Self {
			ConsensusMessage { mmr: value.relay.into(), parachain: value.parachain.into() }
		}
	}

	impl From<crate::sp1_beefy::SP1Beefy::SP1BeefyProof> for Sp1BeefyProof {
		fn from(value: crate::sp1_beefy::SP1Beefy::SP1BeefyProof) -> Self {
			Sp1BeefyProof {
				block_number: value
					.commitment
					.blockNumber
					.try_into()
					.expect("block number out of bounds"),
				validator_set_id: value
					.commitment
					.validatorSetId
					.try_into()
					.expect("validator set id out of bounds"),
				mmr_leaf: value.mmrLeaf.into(),
				headers: value.headers.into_iter().map(Into::into).collect(),
				proof: value.proof.to_vec(),
			}
		}
	}
}

impl From<IntermediateState> for local::IntermediateState {
	fn from(value: IntermediateState) -> Self {
		local::IntermediateState {
			height: local::StateMachineHeight {
				state_machine_id: value
					.stateMachineId
					.try_into()
					.expect("state machine id out of bounds"),
				height: value.height.try_into().expect("state machine height out of bounds"),
			},
			commitment: local::StateCommitment {
				timestamp: value.commitment.timestamp.try_into().expect("timestamp out of bounds"),
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

impl From<router::PostRequest> for crate::handler::Handler::PostRequest {
	fn from(value: router::PostRequest) -> Self {
		Self {
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

impl From<router::PostResponse> for crate::handler::Handler::PostResponse {
	fn from(value: router::PostResponse) -> Self {
		Self {
			request: value.post.into(),
			response: value.response.into(),
			timeoutTimestamp: value.timeout_timestamp,
		}
	}
}

impl TryFrom<PostRequest> for router::PostRequest {
	type Error = anyhow::Error;
	fn try_from(value: PostRequest) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(
				&String::from_utf8(value.source.to_vec()).map_err(|e| anyhow!("{e}"))?,
			)
			.map_err(|err| anyhow!("{err}"))?,
			dest: StateMachine::from_str(
				&String::from_utf8(value.dest.to_vec()).map_err(|e| anyhow!("{e}"))?,
			)
			.map_err(|err| anyhow!("{err}"))?,
			nonce: value.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
			from: value.from.to_vec(),
			to: value.to.to_vec(),
			timeout_timestamp: value.timeoutTimestamp.try_into().map_err(|e| anyhow!("{e}"))?,
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
				let mut address = H160::default();
				address.0.copy_from_slice(&value.from);
				alloy_primitives::Address::from(address.0)
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
				.map(|storage_value| crate::evm_host::MerklePatricia::StorageValue {
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
			EvmHostEvents::GetRequestEvent(get) => {
				Ok(ismp::events::Event::GetRequest(get.try_into()?))
			},
			EvmHostEvents::PostRequestEvent(post) => {
				Ok(ismp::events::Event::PostRequest(post.try_into()?))
			},
			EvmHostEvents::PostResponseEvent(resp) => {
				Ok(ismp::events::Event::PostResponse(router::PostResponse {
					post: router::PostRequest {
						source: StateMachine::from_str(&resp.dest).map_err(|e| anyhow!("{}", e))?,
						dest: StateMachine::from_str(&resp.source).map_err(|e| anyhow!("{}", e))?,
						nonce: resp.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
						from: resp.to.0.to_vec(),
						to: resp.from.0.to_vec(),
						timeout_timestamp: resp
							.timeoutTimestamp
							.try_into()
							.map_err(|e| anyhow!("{e}"))?,
						body: resp.body.to_vec(),
					},
					response: resp.response.to_vec(),
					timeout_timestamp: resp
						.responseTimeoutTimestamp
						.try_into()
						.map_err(|e| anyhow!("{e}"))?,
				}))
			},
			EvmHostEvents::PostRequestHandled(handled) => {
				Ok(ismp::events::Event::PostRequestHandled(ismp::events::RequestResponseHandled {
					commitment: H256(handled.commitment.0),
					relayer: handled.relayer.0.to_vec(),
				}))
			},
			EvmHostEvents::GetRequestHandled(handled) => {
				Ok(ismp::events::Event::GetRequestHandled(ismp::events::RequestResponseHandled {
					commitment: H256(handled.commitment.0),
					relayer: handled.relayer.0.to_vec(),
				}))
			},

			EvmHostEvents::PostResponseHandled(handled) => {
				Ok(ismp::events::Event::PostResponseHandled(ismp::events::RequestResponseHandled {
					commitment: H256(handled.commitment.0),
					relayer: handled.relayer.0.to_vec(),
				}))
			},
			EvmHostEvents::StateMachineUpdated(filter) => {
				Ok(ismp::events::Event::StateMachineUpdated(StateMachineUpdated {
					state_machine_id: ismp::consensus::StateMachineId {
						state_id: StateMachine::from_str(&filter.stateMachineId)
							.map_err(|e| anyhow!("{}", e))?,
						consensus_state_id: Default::default(),
					},
					latest_height: filter.height.try_into().map_err(|e| anyhow!("{e}"))?,
				}))
			},
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
			EvmHostEvents::StateCommitmentVetoed(vetoed) => {
				Ok(ismp::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
					height: ismp::consensus::StateMachineHeight {
						id: StateMachineId {
							state_id: StateMachine::from_str(&vetoed.stateMachineId)
								.map_err(|e| anyhow!("{}", e))?,
							consensus_state_id: Default::default(),
						},
						height: vetoed.height.try_into().map_err(|e| anyhow!("{e}"))?,
					},
					fisherman: vetoed.fisherman.0.to_vec(),
				}))
			},
			EvmHostEvents::StateCommitmentRead(_)
			| EvmHostEvents::HostFrozen(_)
			| EvmHostEvents::HostWithdrawal(_)
			| EvmHostEvents::HostParamsUpdated(_)
			| EvmHostEvents::PostResponseFunded(_)
			| EvmHostEvents::RequestFunded(_) => Err(anyhow!("Unsupported Event!"))?,
		}
	}
}

impl TryFrom<PostRequestEvent> for router::PostRequest {
	type Error = anyhow::Error;

	fn try_from(post: PostRequestEvent) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(&post.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&post.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: post.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
			from: post.from.0.to_vec(),
			to: post.to.0.to_vec(),
			timeout_timestamp: post.timeoutTimestamp.try_into().map_err(|e| anyhow!("{e}"))?,
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
			nonce: get.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
			from: get.from.0.to_vec(),
			keys: get.keys.into_iter().map(|key| key.to_vec()).collect(),
			height: get.height.try_into().map_err(|e| anyhow!("{e}"))?,
			context: get.context.to_vec(),
			timeout_timestamp: get.timeoutTimestamp.try_into().map_err(|e| anyhow!("{e}"))?,
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
