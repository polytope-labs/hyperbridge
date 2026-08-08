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

use crate::ecdsa_beefy::Beefy::IntermediateState;

use alloy_primitives::U256;
use primitive_types::H256;

/// Helper trait for converting primitive types to alloy U256
pub trait ToU256 {
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
		ecdsa_beefy::Beefy::{
			AuthoritySetCommitment, BeefyConsensusProof, BeefyConsensusState, BeefyMmrLeaf,
			Commitment, Parachain, ParachainProof, Payload, RelayChainProof, SignedCommitment,
			Vote,
		},
		sp1_beefy::SP1Beefy::{MiniCommitment, ParachainHeader, PartialBeefyMmrLeaf},
	};
	use alloc::{vec, vec::Vec};
	use alloy_primitives::{Bytes, FixedBytes};
	use beefy_verifier_primitives::{
		ConsensusMessage, ConsensusState, MmrProof, ParachainHeader as BvpParachainHeader,
		ParachainProof as BvpParachainProof, SignatureWithAuthorityIndex,
		SignedCommitment as BvpSignedCommitment, Sp1BeefyProof, TSignature,
	};
	use polkadot_sdk::*;
	use primitive_types::H256;
	use sp_consensus_beefy::{
		mmr::{BeefyNextAuthoritySet, MmrLeafVersion},
		Payload as BeefyPayload,
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
				blockNumber: value.block_number,
				validatorSetId: value.validator_set_id,
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
				version: 0,
				parentNumber: value.parent_number_and_hash.0,
				parentHash: FixedBytes::from(value.parent_number_and_hash.1 .0),
				nextAuthoritySet: Sp1AuthoritySetCommitment {
					id: value.beefy_next_authority_set.id,
					len: value.beefy_next_authority_set.len,
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
					version: 0,
					parentNumber: value.latest_mmr_leaf.parent_number_and_hash.0,
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
				id: value.id,
				len: value.len,
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
			let leaf_index: u64 =
				value.latestMmrLeaf.leafIndex.try_into().expect("mmr leaf index out of bounds");
			let items: Vec<H256> = value.mmrProof.into_iter().map(|h| H256(h.0)).collect();
			let mmr_proof = LeafProof {
				leaf_indices: vec![leaf_index],
				leaf_count: leaf_index.saturating_add(1),
				items,
			};

			MmrProof {
				signed_commitment: BvpSignedCommitment {
					commitment: value.signedCommitment.commitment.into(),
					signatures: value.signedCommitment.votes.into_iter().map(Into::into).collect(),
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
				nonce: H256(value.nonce.0),
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
