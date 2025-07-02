#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
pub use bsc_verifier::primitives::{Mainnet, Testnet};
use bsc_verifier::{
	primitives::{compute_epoch, BscClientUpdate},
	verify_bsc_header, NextValidators, VerificationResult,
};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use evm_state_machine::EvmStateMachine;
use geth_primitives::Header;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId,
	},
	error::Error,
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
			.map_err(|_| Error::Custom("Cannot decode bsc client update".to_string()))?;

		let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		if consensus_state.finalized_height >= bsc_client_update.source_header.number.low_u64() {
			Err(Error::Custom("Expired Update".to_string()))?
		}

		let epoch_length = Pallet::<T>::epoch_length()
			.ok_or_else(|| Error::Custom("Epoch length not set".to_string()))?;
		if let Some(next_validators) = consensus_state.next_validators.clone() {
			if bsc_client_update.attested_header.number.low_u64() % epoch_length >=
				(consensus_state.current_validators.len() as u64 / 2)
			{
				// Sanity check
				// During authority set rotation, the source header must be from the same epoch as
				// the attested header
				let epoch =
					compute_epoch(bsc_client_update.attested_header.number.low_u64(), epoch_length);
				let source_header_epoch =
					compute_epoch(bsc_client_update.source_header.number.low_u64(), epoch_length);
				if source_header_epoch != epoch {
					Err(Error::Custom("The Source Header must be from the same epoch with the attested epoch during an authority set rotation".to_string()))?
				}
				consensus_state.current_validators = next_validators.validators;
				consensus_state.next_validators = None;
				consensus_state.current_epoch = epoch;
			}
		}

		let VerificationResult { hash, finalized_header, next_validators } =
			verify_bsc_header::<H, C>(
				&consensus_state.current_validators,
				bsc_client_update,
				epoch_length,
			)
			.map_err(|e| Error::Custom(e.to_string()))?;

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
		let bsc_client_update_1 = BscClientUpdate::decode(&mut &proof_1[..]).map_err(|_| {
			Error::Custom("Cannot decode bsc client update for proof 1".to_string())
		})?;

		let bsc_client_update_2 = BscClientUpdate::decode(&mut &proof_2[..]).map_err(|_| {
			Error::Custom("Cannot decode bsc client update for proof 2".to_string())
		})?;

		let header_1 = bsc_client_update_1.attested_header.clone();
		let header_2 = bsc_client_update_2.attested_header.clone();

		if header_1.number != header_2.number {
			Err(Error::Custom("Invalid Fraud proof".to_string()))?
		}

		let header_1_hash = Header::from(&header_1).hash::<H>();
		let header_2_hash = Header::from(&header_2).hash::<H>();

		if header_1_hash == header_2_hash {
			return Err(Error::Custom("Invalid Fraud proof".to_string()));
		}

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;
		let epoch_length = Pallet::<T>::epoch_length()
			.ok_or_else(|| Error::Custom("Epoch length not set".to_string()))?;
		let _ = verify_bsc_header::<H, C>(
			&consensus_state.current_validators,
			bsc_client_update_1,
			epoch_length,
		)
		.map_err(|_| Error::Custom("Failed to verify first header".to_string()))?;

		let _ = verify_bsc_header::<H, C>(
			&consensus_state.current_validators,
			bsc_client_update_2,
			epoch_length,
		)
		.map_err(|_| Error::Custom("Failed to verify second header".to_string()))?;

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
			state_machine =>
				Err(Error::Custom(alloc::format!("Unsupported state machine: {state_machine:?}"))),
		}
	}
}
