#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
pub use bsc_verifier::primitives::{Mainnet, Testnet};
use bsc_verifier::{
	ensure_finalized_epoch_consistent,
	primitives::{
		compute_epoch, parse_extra, should_promote_validators, staged_set_activates_next_epoch,
		BscClientUpdate,
	},
	verify_bsc_header, Error, NextValidators, VerificationResult,
};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use evm_state_machine::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId,
	},
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use polkadot_sdk::*;
use sp_core::H256;
use sync_committee_primitives::constants::BlsPublicKey;

pub mod pallet;

use pallet::Pallet;

pub const BSC_CONSENSUS_ID: ConsensusStateId = *b"BSCP";

const BSC_CHAIN_ID: u32 = 56;
const BSC_TESTNET_CHAIN_ID: u32 = 97;

#[derive(codec::Encode, codec::Decode, Debug, Default, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	pub current_validators: Vec<BlsPublicKey>,
	pub next_validators: Option<NextValidators>,
	pub finalized_height: u64,
	pub finalized_hash: H256,
	pub current_epoch: u64,
	pub chain_id: u32,
}

pub struct BscClient<
	H: IsmpHost,
	T: pallet_ismp_host_executive::Config,
	C: bsc_verifier::primitives::Config,
>(PhantomData<(H, T, C)>);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config, C: bsc_verifier::primitives::Config>
	Default for BscClient<H, T, C>
{
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config, C: bsc_verifier::primitives::Config> Clone
	for BscClient<H, T, C>
{
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		T: pallet_ismp_host_executive::Config + crate::pallet::Config,
		C: bsc_verifier::primitives::Config,
	> ConsensusClient for BscClient<H, T, C>
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), ismp::error::Error> {
		let bsc_client_update =
			BscClientUpdate::decode(&mut &proof[..]).map_err(|_| Error::DecodeBscClientUpdate)?;

		let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::DecodeConsensusState)?;

		if consensus_state.finalized_height >= bsc_client_update.source_header.number.low_u64() {
			Err(Error::ExpiredUpdate {
				current: consensus_state.finalized_height,
				update: bsc_client_update.source_header.number.low_u64(),
			})?
		}

		let epoch_length = Pallet::<T>::epoch_length().ok_or(Error::EpochLengthNotSet)?;
		if let Some(next_validators) = consensus_state.next_validators.clone() {
			let source_number = bsc_client_update.source_header.number.low_u64();
			// Promote the pending set only when the finalized, BLS-signed `source_header` has
			// reached the recorded `rotation_block` in the epoch where that set activates. Gating
			// on the signed source height (not the unsigned attested height) is what keeps a
			// compromised outgoing set from staying in charge past its rotation — see
			// [`should_promote_validators`]. The relayer mirrors this exact predicate.
			if should_promote_validators(
				source_number,
				next_validators.rotation_block,
				epoch_length,
			) {
				consensus_state.current_validators = next_validators.validators;
				consensus_state.next_validators = None;
				consensus_state.current_epoch = compute_epoch(source_number, epoch_length);
			}
		}

		let VerificationResult { hash, finalized_header, next_validators } =
			verify_bsc_header::<H, C>(
				&consensus_state.current_validators,
				bsc_client_update,
				epoch_length,
			)?;

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: finalized_header.timestamp,
				overlay_root: None,
				state_root: finalized_header.state_root,
			},
			height: finalized_header.number.low_u64(),
		};
		consensus_state.finalized_hash = hash;

		if let Some(next_validators) = next_validators {
			// A freshly staged set must belong to the epoch immediately after the one the client
			// currently holds. Binding the ancestry to `source_header`'s own epoch boundary (in
			// the verifier) still lets a caller stage the *current* epoch's own set as "next" —
			// a source in epoch E carrying a full-epoch ancestry back to E's boundary extracts
			// E's set, with a `rotation_block` already in the past. Such a set can never promote
			// (its rotation epoch is E, which finality has left), and once finality crosses into
			// E+1 it strands the client until governance resets it. Requiring the staged set's
			// activation epoch to equal `current_epoch + 1` closes that path while leaving the
			// honest flow (stage epoch E+1's set while in epoch E) untouched.
			if !staged_set_activates_next_epoch(
				next_validators.rotation_block,
				consensus_state.current_epoch,
				epoch_length,
			) {
				Err(Error::UnexpectedNextValidatorsEpoch {
					staged_epoch: compute_epoch(next_validators.rotation_block, epoch_length),
					current_epoch: consensus_state.current_epoch,
				})?
			}
			consensus_state.next_validators = Some(next_validators);
		}
		consensus_state.finalized_height = finalized_header.number.low_u64();

		// Reject any update that would leave `finalized_height` running ahead of the validator set
		// the client holds — this would permanently strand the client (see
		// `ensure_finalized_epoch_consistent`). The finalized height may only cross an epoch
		// boundary via a validator-set-staging (sync) update.
		ensure_finalized_epoch_consistent(
			consensus_state.finalized_height,
			consensus_state.current_epoch,
			consensus_state.next_validators.is_some(),
			epoch_length,
		)?;

		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Evm(consensus_state.chain_id),
				consensus_state_id,
			},
			vec![state_commitment],
		);

		Ok((consensus_state.encode(), state_machine_map))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), ismp::error::Error> {
		let bsc_client_update_1 =
			BscClientUpdate::decode(&mut &proof_1[..]).map_err(|_| Error::DecodeBscClientUpdate)?;

		let bsc_client_update_2 =
			BscClientUpdate::decode(&mut &proof_2[..]).map_err(|_| Error::DecodeBscClientUpdate)?;

		let header_1 = bsc_client_update_1.attested_header.clone();
		let header_2 = bsc_client_update_2.attested_header.clone();

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::DecodeConsensusState)?;
		let epoch_length = Pallet::<T>::epoch_length().ok_or(Error::EpochLengthNotSet)?;

		// Authenticate both updates against the trusted validator set: this verifies
		// the BLS aggregate signature over each update's `vote_data`.
		let _ = verify_bsc_header::<H, C>(
			&consensus_state.current_validators,
			bsc_client_update_1,
			epoch_length,
		)?;

		let _ = verify_bsc_header::<H, C>(
			&consensus_state.current_validators,
			bsc_client_update_2,
			epoch_length,
		)?;

		// The fraud proof must be bound to the BLS-signed `vote_data`, never to the
		// `attested_header` itself. The header's non-vote fields (e.g. `state_root`,
		// `parent_hash`, `receipts_root`) are not covered by the signature, so a
		// single genuine attestation can be cloned into two distinct-hashing headers
		// that carry the same vote. A genuine BSC equivocation is a slashable double
		// vote: two quorum-signed votes for the same target block number but
		// different target hashes.
		let vote_1 =
			parse_extra::<H, C>(&header_1).map_err(|_| Error::InvalidFraudProof)?.vote_data;
		let vote_2 =
			parse_extra::<H, C>(&header_2).map_err(|_| Error::InvalidFraudProof)?.vote_data;

		if vote_1.target_number != vote_2.target_number {
			Err(Error::InvalidFraudProof)?
		}

		if vote_1.target_hash == vote_2.target_hash {
			return Err(Error::InvalidFraudProof.into());
		}

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		BSC_CONSENSUS_ID
	}

	fn state_machine(
		&self,
		id: ismp::host::StateMachine,
	) -> Result<Box<dyn StateMachineClient>, ismp::error::Error> {
		match id {
			StateMachine::Evm(chain_id)
				if chain_id == BSC_CHAIN_ID || chain_id == BSC_TESTNET_CHAIN_ID =>
				Ok(Box::new(<EvmStateMachine<H, T>>::default())),
			state_machine => Err(Error::UnsupportedStateMachine(state_machine).into()),
		}
	}
}
