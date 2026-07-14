#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
pub use bsc_verifier::primitives::{Mainnet, Testnet};
use bsc_verifier::{
	primitives::{compute_epoch, parse_extra, BscClientUpdate},
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
		let bsc_client_update = BscClientUpdate::decode(&mut &proof[..])
			.map_err(|_| Error::DecodeBscClientUpdate)?;

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
			let attested_number = bsc_client_update.attested_header.number.low_u64();
			let attested_epoch = compute_epoch(attested_number, epoch_length);
			let rotation_epoch = compute_epoch(next_validators.rotation_block, epoch_length);
			// Promote the pending validator set only when the submitted update is in the
			// specific epoch where that set is scheduled to activate, and the attested
			// header has reached the recorded `rotation_block`. The previous rule —
			// "any update whose `attested.number % epoch_length` is past the rotation
			// midpoint" — promoted the pending set in any later epoch, so an attacker
			// holding the keys of a stale `next_validators` (e.g. retired or compromised
			// validators) could submit an update many epochs later, get their set
			// promoted to `current_validators`, and then have their forged
			// `source_header`'s `state_root` accepted as a BSC state commitment. Binding
			// rotation to the recorded `rotation_block`'s epoch prevents that reuse.
			if attested_epoch == rotation_epoch && attested_number >= next_validators.rotation_block {
				// During authority set rotation, the source header must be from the same epoch as
				// the attested header.
				let source_header_epoch =
					compute_epoch(bsc_client_update.source_header.number.low_u64(), epoch_length);
				if source_header_epoch != attested_epoch {
					Err(Error::SourceHeaderEpochMismatch {
						attested_epoch,
						source_epoch: source_header_epoch,
					})?
				}
				consensus_state.current_validators = next_validators.validators;
				consensus_state.next_validators = None;
				consensus_state.current_epoch = attested_epoch;
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
			consensus_state.next_validators = Some(next_validators);
		}
		consensus_state.finalized_height = finalized_header.number.low_u64();

		// Invariant: `finalized_height` must never run ahead of the validator set the client
		// holds. `current_validators` sign for `current_epoch` and remain valid into the first
		// half of `current_epoch + 1`, but the client may only *rely* on that overlap once the
		// next set has been staged (`next_validators`) so the rotation can subsequently be
		// enacted. An update that finalizes a header in a new epoch without staging that rotation
		// leaves `current_epoch`/`current_validators` a full epoch behind `finalized_height`; the
		// relayer derives its sync target from `max(epoch(finalized_height), current_epoch)`, so it
		// would skip the un-staged epoch forever and the client would be permanently stuck (also a
		// griefing vector). Reject such updates: the finalized height may only cross an epoch
		// boundary via a validator-set-staging (sync) update, and by at most one epoch.
		let finalized_epoch = compute_epoch(consensus_state.finalized_height, epoch_length);
		let max_finalized_epoch =
			consensus_state.current_epoch + consensus_state.next_validators.is_some() as u64;
		if finalized_epoch > max_finalized_epoch {
			Err(Error::StaleValidatorSet {
				finalized_epoch,
				current_epoch: consensus_state.current_epoch,
				next_validators_staged: consensus_state.next_validators.is_some(),
			})?
		}

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
		let vote_1 = parse_extra::<H, C>(&header_1)
			.map_err(|_| Error::InvalidFraudProof)?
			.vote_data;
		let vote_2 = parse_extra::<H, C>(&header_2)
			.map_err(|_| Error::InvalidFraudProof)?
			.vote_data;

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
