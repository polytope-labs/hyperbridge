use std::marker::PhantomData;

use anyhow::anyhow;
use bls::{point_to_pubkey, types::G1ProjectivePoint};
use log::trace;
use primitive_types::H256;
use reqwest::{Client, Url};
use reqwest_chain::ChainMiddleware;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use ssz_rs::{Merkleized, Node};
use tracing::instrument;

use sync_committee_primitives::{
	consensus_types::{BeaconBlock, BeaconBlockHeader, BeaconState, Checkpoint, Validator},
	constants::{
		BlsPublicKey, Config, Root, BYTES_PER_LOGS_BLOOM, EPOCHS_PER_HISTORICAL_VECTOR,
		EPOCHS_PER_SLASHINGS_VECTOR, HISTORICAL_ROOTS_LIMIT, MAX_ATTESTATIONS,
		MAX_ATTESTER_SLASHINGS, MAX_BLS_TO_EXECUTION_CHANGES, MAX_BYTES_PER_TRANSACTION,
		MAX_COMMITTEES_PER_SLOT, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD, MAX_DEPOSITS,
		MAX_DEPOSIT_REQUESTS_PER_PAYLOAD, MAX_EXTRA_DATA_BYTES, MAX_PROPOSER_SLASHINGS,
		MAX_TRANSACTIONS_PER_PAYLOAD, MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS,
		MAX_WITHDRAWALS_PER_PAYLOAD, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
		PENDING_CONSOLIDATIONS_LIMIT, PENDING_DEPOSITS_LIMIT, PENDING_PARTIAL_WITHDRAWALS_LIMIT,
		SLOTS_PER_HISTORICAL_ROOT, SYNC_COMMITTEE_SIZE, VALIDATOR_REGISTRY_LIMIT,
	},
	deneb::MAX_BLOB_COMMITMENTS_PER_BLOCK,
	types::{
		ExecutionPayloadProof, FinalityProof, SyncCommitteeUpdate, VerifierState,
		VerifierStateUpdate,
	},
	util::{compute_sync_committee_period_at_slot, should_have_sync_committee_update},
};
use sync_committee_verifier::crypto::pubkey_to_projective;

use crate::{
	middleware::SwitchProviderMiddleware,
	responses::{
		finality_checkpoint_response::FinalityCheckpoint,
		sync_committee_response::NodeSyncCommittee,
	},
	routes::*,
};

pub mod middleware;
#[warn(unused_imports)]
#[warn(unused_variables)]
pub mod responses;
pub mod routes;

#[cfg(test)]
mod test;

pub type BeaconStateType<const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> = BeaconState<
	SLOTS_PER_HISTORICAL_ROOT,
	HISTORICAL_ROOTS_LIMIT,
	ETH1_DATA_VOTES_BOUND,
	VALIDATOR_REGISTRY_LIMIT,
	EPOCHS_PER_HISTORICAL_VECTOR,
	EPOCHS_PER_SLASHINGS_VECTOR,
	SYNC_COMMITTEE_SIZE,
	BYTES_PER_LOGS_BLOOM,
	MAX_EXTRA_DATA_BYTES,
	PENDING_DEPOSITS_LIMIT,
	PENDING_CONSOLIDATIONS_LIMIT,
	PENDING_PARTIAL_WITHDRAWALS_LIMIT,
	PROPOSER_LOOK_AHEAD_LIMIT
>;

pub struct SyncCommitteeProver<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> {
	pub primary_url: String,
	pub providers: Vec<String>,
	pub client: ClientWithMiddleware,
	pub phantom: PhantomData<C>,
}

impl<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> Clone
	for SyncCommitteeProver<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>
{
	fn clone(&self) -> Self {
		Self {
			primary_url: self.primary_url.clone(),
			client: self.client.clone(),
			providers: self.providers.clone(),
			phantom: PhantomData,
		}
	}
}

impl<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> SyncCommitteeProver<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT> {
	pub fn new(providers: Vec<String>) -> Self {
		let client = ClientBuilder::new(Client::new())
			.with(ChainMiddleware::new(SwitchProviderMiddleware::_new(providers.clone())))
			.build();

		SyncCommitteeProver::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT> {
			primary_url: providers.get(0).expect("There must be atleast one provider").clone(),
			providers,
			client,
			phantom: PhantomData,
		}
	}
	#[instrument(level = "trace", target = "sync-committee-prover", skip(self))]
	pub async fn fetch_finalized_checkpoint(
		&self,
		state_id: Option<&str>,
	) -> Result<FinalityCheckpoint, anyhow::Error> {
		trace!(target: "sync-committee-prover", "Fetching finalized checkpoint {state_id:?}");
		let full_url = self.generate_route(&finality_checkpoints(state_id.unwrap_or("head")))?;
		let response = self
			.client
			.get(full_url)
			.send()
			.await
			.map_err(|e| anyhow!("Failed to fetch finalized checkpoint due to error {e:?}"))?;

		let response_data = response
			.json::<responses::finality_checkpoint_response::Response>()
			.await
			.map_err(|e| anyhow!("Failed to fetch finalized checkpoint due to error {e:?}"))?;
		Ok(response_data.data)
	}

	#[instrument(level = "trace", target = "sync-committee-prover", skip(self))]
	pub async fn fetch_header(&self, block_id: &str) -> Result<BeaconBlockHeader, anyhow::Error> {
		trace!(target: "sync-committee-prover", "Fetching Header {block_id}");
		let path = header_route(block_id);
		let full_url = self.generate_route(&path)?;
		let response =
			self.client.get(full_url).send().await.map_err(|e| {
				anyhow!("Failed to fetch header with id {block_id} due to error {e:?}")
			})?;

		let response_data = response
			.json::<responses::beacon_block_header_response::Response>()
			.await
			.map_err(|e| anyhow!("Failed to fetch header with id {block_id} due to error {e:?}"))?;

		let beacon_block_header = response_data.data.header.message;

		Ok(beacon_block_header)
	}

	#[instrument(level = "trace", target = "sync-committee-prover", skip(self))]
	pub async fn fetch_block(
		&self,
		block_id: &str,
	) -> Result<
		BeaconBlock<
			MAX_PROPOSER_SLASHINGS,
			MAX_VALIDATORS_PER_COMMITTEE,
			MAX_ATTESTER_SLASHINGS,
			MAX_ATTESTATIONS,
			MAX_DEPOSITS,
			MAX_VOLUNTARY_EXITS,
			SYNC_COMMITTEE_SIZE,
			BYTES_PER_LOGS_BLOOM,
			MAX_EXTRA_DATA_BYTES,
			MAX_BYTES_PER_TRANSACTION,
			MAX_TRANSACTIONS_PER_PAYLOAD,
			MAX_WITHDRAWALS_PER_PAYLOAD,
			MAX_BLS_TO_EXECUTION_CHANGES,
			MAX_BLOB_COMMITMENTS_PER_BLOCK,
			MAX_COMMITTEES_PER_SLOT,
			MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
			MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
			MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
		>,
		anyhow::Error,
	> {
		trace!(target: "sync-committee-prover", "Fetching block {block_id}");
		let path = block_route(block_id);
		let full_url = self.generate_route(&path)?;

		let response =
			self.client.get(full_url).send().await.map_err(|e| {
				anyhow!("Failed to fetch block with id {block_id} due to error {e:?}")
			})?;

		let response_data = response
			.json::<responses::beacon_block_response::Response>()
			.await
			.map_err(|e| anyhow!("Failed to fetch block with id {block_id} due to error {e:?}"))?;

		let beacon_block = response_data.data.message;

		Ok(beacon_block)
	}

	pub async fn fetch_sync_committee(
		&self,
		state_id: &str,
	) -> Result<NodeSyncCommittee, anyhow::Error> {
		let path = sync_committee_route(state_id);
		let full_url = self.generate_route(&path)?;

		let response = self.client.get(full_url).send().await?;

		let response_data = response.json::<responses::sync_committee_response::Response>().await?;

		let sync_committee = response_data.data;

		Ok(sync_committee)
	}

	pub async fn fetch_validator(
		&self,
		state_id: &str,
		validator_index: &str,
	) -> Result<Validator, anyhow::Error> {
		let path = validator_route(state_id, validator_index);
		let full_url = self.generate_route(&path)?;

		let response = self.client.get(full_url).send().await?;

		let response_data = response.json::<responses::validator_response::Response>().await?;

		let validator = response_data.data.validator;

		Ok(validator)
	}

	#[instrument(level = "trace", target = "sync-committee-prover", skip(self))]
	pub async fn fetch_beacon_state(
		&self,
		state_id: &str,
	) -> Result<BeaconStateType<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>, anyhow::Error> {
		trace!(target: "sync-committee-prover", "Fetching beacon state {state_id}");
		let path = beacon_state_route(state_id);
		let full_url = self.generate_route(&path)?;

		let response = self.client.get(full_url).send().await.map_err(|e| {
			// Rpc Errors can be too long sometimes, let's truncate them
			let mut e = e.to_string();
			e.truncate(1024);
			anyhow!("Failed to fetch beacon state with id {state_id} due to error {e:?}")
		})?;

		let response_data = response
			.json::<responses::beacon_state_response::Response<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>>()
			.await
			.map_err(|e| {
				anyhow!("Failed to fetch beacon state with id {state_id} due to error {e:?}")
			})?;

		let beacon_state = response_data.data;

		Ok(beacon_state)
	}

	fn generate_route(&self, path: &str) -> Result<Url, anyhow::Error> {
		let url = Url::parse(&format!("{}{}", self.primary_url.clone(), path))?;
		Ok(url)
	}

	/// Fetches the latest finality update that can be verified by (state_period..=state_period+1)
	/// latest_block_id is an optional block id where to start the signature block search, if absent
	/// we use `head`
	pub async fn fetch_light_client_update(
		&self,
		mut client_state: VerifierState,
		finality_checkpoint: Checkpoint,
		latest_block_id: Option<&str>,
	) -> Result<Option<VerifierStateUpdate>, anyhow::Error> {
		if finality_checkpoint.root == Node::default() ||
			client_state.latest_finalized_epoch >= finality_checkpoint.epoch
		{
			trace!(target: "sync-committee-prover", "No new epoch finalized yet {}", finality_checkpoint.epoch);
			return Ok(None);
		}

		trace!(target: "sync-committee-prover", "A new epoch has been finalized {}", finality_checkpoint.epoch);
		// Find the highest block with the a threshhold number of sync committee signatures
		let latest_header = self.fetch_header(latest_block_id.unwrap_or("head")).await?;
		let latest_root = latest_header.clone().hash_tree_root()?;
		let get_block_id = |root: Root| {
			let mut block_id = hex::encode(root.0.to_vec());
			block_id.insert_str(0, "0x");
			block_id
		};
		let mut block = self.fetch_block(&get_block_id(latest_root)).await?;
		let min_signatures = ((2 * SYNC_COMMITTEE_SIZE) / 3) + 1;
		let state_period = client_state.state_period;
		loop {
			// Some checks on the epoch finalized by the signature block
			let parent_root = block.parent_root;
			let parent_block_id = get_block_id(parent_root);
			let parent_block = self.fetch_block(&parent_block_id).await?;
			let parent_state_id = get_block_id(parent_block.state_root);
			let parent_block_finality_checkpoint =
				self.fetch_finalized_checkpoint(Some(&parent_state_id)).await?.finalized;
			if parent_block_finality_checkpoint.epoch <= client_state.latest_finalized_epoch {
				trace!(target: "sync-committee-prover", "A block with valid sync committe signature was not found for epoch {:?}", finality_checkpoint.epoch);
				return Ok(None);
			}

			let num_signatures = block.body.sync_aggregate.sync_committee_bits.count_ones();

			let signature_period = compute_sync_committee_period_at_slot::<C>(block.slot);

			if num_signatures >= min_signatures &&
				(state_period..=state_period + 1).contains(&signature_period) &&
				parent_block_finality_checkpoint.epoch > client_state.latest_finalized_epoch
			{
				break;
			}
			block = parent_block;
		}

		let attested_block_id = get_block_id(block.parent_root);
		let attested_header = self.fetch_header(&attested_block_id).await?;
		let mut attested_state =
			self.fetch_beacon_state(&get_block_id(attested_header.state_root)).await?;
		if attested_state.finalized_checkpoint.root == Node::default() {
			return Ok(None);
		}
		let finalized_block_id = get_block_id(attested_state.finalized_checkpoint.root);
		let finalized_header = self.fetch_header(&finalized_block_id).await?;
		let mut finalized_state =
			self.fetch_beacon_state(&get_block_id(finalized_header.state_root)).await?;
		let finality_proof = FinalityProof {
			epoch: attested_state.finalized_checkpoint.epoch,
			finality_branch: prove_finalized_header::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>(
				&mut attested_state,
			)?,
		};

		let execution_payload_proof =
			prove_execution_payload::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>(&mut finalized_state)?;

		let signature_period = compute_sync_committee_period_at_slot::<C>(block.slot);
		let client_state_next_sync_committee_root =
			client_state.next_sync_committee.hash_tree_root()?;
		let attested_state_current_sync_committee_root =
			attested_state.current_sync_committee.hash_tree_root()?;
		let sync_committee_update =
            // We must make sure we switch the sync comittee only when the finalized header has changed sync committees
            if should_have_sync_committee_update(state_period, signature_period) && client_state_next_sync_committee_root == attested_state_current_sync_committee_root {
                let sync_committee_proof = prove_sync_committee_update::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>(&mut attested_state)?;
                Some(SyncCommitteeUpdate {
                    next_sync_committee: attested_state.next_sync_committee,
                    next_sync_committee_branch: sync_committee_proof,
                })
            } else {
                None
            };

		// construct light client
		let light_client_update = VerifierStateUpdate {
			attested_header,
			sync_committee_update,
			finalized_header,
			execution_payload: execution_payload_proof,
			finality_proof,
			sync_aggregate: block.body.sync_aggregate,
			signature_slot: block.slot,
		};

		trace!(target: "sync-committee-prover", "got light client update");

		Ok(Some(light_client_update))
	}

	#[instrument(level = "trace", target = "sync-committee-prover", skip(self))]
	pub async fn latest_update_for_period(
		&self,
		period: u64,
	) -> Result<VerifierStateUpdate, anyhow::Error> {
		trace!(target: "sync-committee-prover", "latest_update_for_period {period}");
		let mut higest_slot_in_epoch = ((period * C::EPOCHS_PER_SYNC_COMMITTEE_PERIOD) *
			C::SLOTS_PER_EPOCH) +
			(C::EPOCHS_PER_SYNC_COMMITTEE_PERIOD * C::SLOTS_PER_EPOCH) -
			1;
		let mut count = 0;
		// Some slots are empty so we'll use a loop to fetch the highest available slot in an epoch
		let mut block = loop {
			// Prevent an infinite loop
			if count == 100 {
				log::trace!(target: "sync-committee-prover", "Prover could not find a suitable block for the sync committee: {period}, syncing will fail");
				return Err(anyhow!("Error fetching blocks from selected epoch"));
			}

			if let Ok(block) = self.fetch_block(&higest_slot_in_epoch.to_string()).await {
				break block;
			} else {
				higest_slot_in_epoch -= 1;
				count += 1;
			}
		};
		let min_signatures = ((2 * SYNC_COMMITTEE_SIZE) / 3) + 1;
		let get_block_id = |root: Root| {
			let mut block_id = hex::encode(root.0.to_vec());
			block_id.insert_str(0, "0x");
			block_id
		};
		loop {
			let num_signatures = block.body.sync_aggregate.sync_committee_bits.count_ones();
			if num_signatures >= min_signatures {
				break;
			}

			let parent_root = block.parent_root;
			let parent_block_id = get_block_id(parent_root);
			let parent_block = self.fetch_block(&parent_block_id).await?;

			block = parent_block;
		}

		let attested_block_id = get_block_id(block.parent_root);

		let attested_header = self.fetch_header(&attested_block_id).await?;
		let mut attested_state =
			self.fetch_beacon_state(&get_block_id(attested_header.state_root)).await?;
		let finalized_block_id = get_block_id(attested_state.finalized_checkpoint.root);
		let finalized_header = self.fetch_header(&finalized_block_id).await?;
		let mut finalized_state =
			self.fetch_beacon_state(&get_block_id(finalized_header.state_root)).await?;
		let finality_proof = FinalityProof {
			epoch: attested_state.finalized_checkpoint.epoch,
			finality_branch: prove_finalized_header::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>(
				&mut attested_state,
			)?,
		};

		let execution_payload_proof =
			prove_execution_payload::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>(&mut finalized_state)?;

		let sync_committee_update = {
			let sync_committee_proof =
				prove_sync_committee_update::<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>(&mut attested_state)?;
			Some(SyncCommitteeUpdate {
				next_sync_committee: attested_state.next_sync_committee,
				next_sync_committee_branch: sync_committee_proof,
			})
		};

		// construct light client
		let light_client_update = VerifierStateUpdate {
			attested_header,
			sync_committee_update,
			finalized_header,
			execution_payload: execution_payload_proof,
			finality_proof,
			sync_aggregate: block.body.sync_aggregate,
			signature_slot: block.slot,
		};

		Ok(light_client_update)
	}
}

#[instrument(level = "trace", target = "sync-committee-prover", skip_all)]
pub fn prove_execution_payload<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize>(
	beacon_state: &mut BeaconStateType<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>,
) -> anyhow::Result<ExecutionPayloadProof> {
	trace!(target: "sync-committee-prover", "Proving execution payload");
	let indices = [
		C::EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize,
		C::EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize,
		C::EXECUTION_PAYLOAD_TIMESTAMP_INDEX as usize,
	];
	// generate multi proofs
	let multi_proof = ssz_rs::generate_proof(
		&mut beacon_state.latest_execution_payload_header,
		indices.as_slice(),
	)?;

	trace!(target: "sync-committee-prover", "finished proving execution payload");

	Ok(ExecutionPayloadProof {
		state_root: H256::from_slice(
			beacon_state.latest_execution_payload_header.state_root.as_slice(),
		),
		block_number: beacon_state.latest_execution_payload_header.block_number,
		timestamp: beacon_state.latest_execution_payload_header.timestamp,
		multi_proof,
		execution_payload_branch: ssz_rs::generate_proof(
			beacon_state,
			&[C::EXECUTION_PAYLOAD_INDEX as usize],
		)?,
	})
}

#[instrument(level = "trace", target = "sync-committee-prover", skip_all)]
pub fn prove_sync_committee_update<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize>(
	state: &mut BeaconStateType<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>,
) -> anyhow::Result<Vec<Node>> {
	trace!(target: "sync-committee-prover", "Proving sync committee update");
	let proof = ssz_rs::generate_proof(state, &[C::NEXT_SYNC_COMMITTEE_INDEX as usize])?;
	Ok(proof)
}

#[instrument(level = "trace", target = "sync-committee-prover", skip_all)]
pub fn prove_finalized_header<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize>(
	state: &mut BeaconStateType<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>,
) -> anyhow::Result<Vec<Node>> {
	trace!(target: "sync-committee-prover", "Proving finalized head");
	let indices = [C::FINALIZED_ROOT_INDEX as usize];
	let proof = ssz_rs::generate_proof(state, indices.as_slice())?;

	Ok(proof)
}

pub fn eth_aggregate_public_keys(points: &[BlsPublicKey]) -> anyhow::Result<BlsPublicKey> {
	let points = points
		.iter()
		.map(|point| pubkey_to_projective(point))
		.collect::<Result<Vec<_>, _>>()?;
	let aggregate = points
		.into_iter()
		.fold(G1ProjectivePoint::default(), |acc, g1_point| acc + g1_point);
	let public_key = point_to_pubkey(aggregate.into());

	let bls_public_key =
		BlsPublicKey::try_from(public_key.as_slice()).map_err(|e| anyhow!("{:?}", e))?;

	Ok(bls_public_key)
}
