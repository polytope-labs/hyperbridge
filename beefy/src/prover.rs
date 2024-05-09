use std::{collections::HashMap, time::Duration};

use anyhow::anyhow;
use beefy_prover::{relay::fetch_latest_beefy_justification, runtime};
use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use ethabi::ethereum_types::H256;
use ethers::abi::AbiEncode;
use futures::StreamExt;
use ismp::events::Event;
use ismp_solidity_abi::beefy::BeefyConsensusProof;
use pallet_ismp_rpc::{BlockNumberOrHash, EventWithMetadata};
use serde::{Deserialize, Serialize};
use sp_consensus_beefy::{
	ecdsa_crypto::Signature, known_payloads::MMR_ROOT_ID, mmr::BeefyNextAuthoritySet,
	SignedCommitment, VersionedFinalityProof,
};
use sp_runtime::traits::Keccak256;
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::MultiSignature,
	rpc_params, OnlineClient,
};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::SubstrateClient;

use zk_beefy::Network;

use crate::{extract_para_id, VALIDATOR_SET_ID_KEY};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyProverConfig {
	/// Minimum height that must be enacted before we prove finality for new messages
	pub minimum_finalization_height: u64,
}

/// Beefy prover, can either produce zk proofs or naive proofs
#[derive(Clone)]
pub enum Prover<R: subxt::Config, P: subxt::Config> {
	// The naive prover
	Naive(beefy_prover::Prover<R, P>),
	// zk prover
	ZK(zk_beefy::Prover<R, P>),
}

pub struct ProverConfig {
	/// RPC ws url for a relay chain
	pub relay_rpc_ws: String,
	/// RPC ws url for the parachain
	pub para_rpc_ws: String,
	/// para Id for the parachain
	pub para_ids: Vec<u32>,
	/// The intended network for zk beefy
	pub zk_beefy: Option<Network>,
	/// Maximum size in bytes for the rpc payloads, both requests & responses.
	pub max_rpc_payload_size: Option<u32>,
}

impl<R, P> Prover<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	pub async fn new(config: ProverConfig) -> Result<Self, anyhow::Error> {
		let max_rpc_payload_size = config.max_rpc_payload_size.unwrap_or(15 * 1024 * 1024);
		let relay_chain =
			subxt_utils::client::ws_client::<R>(&config.relay_rpc_ws, max_rpc_payload_size).await?;
		let parachain =
			subxt_utils::client::ws_client::<P>(&config.para_rpc_ws, max_rpc_payload_size).await?;
		let header = relay_chain
			.rpc()
			.header(None)
			.await?
			.ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;
		let key = runtime::storage().mmr().number_of_leaves();
		let leaves = relay_chain
			.storage()
			.at(header.hash())
			.fetch(&key)
			.await?
			.ok_or_else(|| anyhow!("Number of mmr leaves is empty"))?;

		let prover = beefy_prover::Prover {
			beefy_activation_block: (header.number().into() - leaves) as u32,
			relay: relay_chain,
			para: parachain,
			para_ids: config.para_ids,
		};

		let prover = if let Some(network) = &config.zk_beefy {
			Prover::ZK(zk_beefy::Prover::new(prover, network.clone())?)
		} else {
			Prover::Naive(prover)
		};

		Ok(prover)
	}

	/// Return the inner prover
	pub fn inner(&self) -> &beefy_prover::Prover<R, P> {
		match self {
			Prover::ZK(ref p) => &p.inner,
			Prover::Naive(ref p) => p,
		}
	}

	/// Construct a beefy client state to be submitted to the counterparty chain
	pub async fn query_initial_consensus_state(
		&self,
		hash: Option<R::Hash>,
	) -> Result<ConsensusState, anyhow::Error> {
		let inner = self.inner();
		let latest_finalized_head = match hash {
			Some(hash) => hash,
			None => inner.relay.rpc().request("beefy_getFinalizedHead", rpc_params!()).await?,
		};
		let (signed_commitment, latest_beefy_finalized) =
			fetch_latest_beefy_justification(&inner.relay, latest_finalized_head).await?;

		// Encoding and decoding to fix dependency version conflicts
		let next_authority_set = {
			let key = runtime::storage().beefy_mmr_leaf().beefy_next_authorities();
			let next_authority_set = inner
				.relay
				.storage()
				.at(latest_beefy_finalized)
				.fetch(&key)
				.await?
				.expect("Should retrieve next authority set")
				.encode();
			BeefyNextAuthoritySet::decode(&mut &*next_authority_set)
				.expect("Should decode next authority set correctly")
		};

		let current_authority_set = {
			let key = runtime::storage().beefy_mmr_leaf().beefy_authorities();
			let authority_set = inner
				.relay
				.storage()
				.at(latest_beefy_finalized)
				.fetch(&key)
				.await?
				.expect("Should retrieve next authority set")
				.encode();
			BeefyNextAuthoritySet::decode(&mut &*authority_set)
				.expect("Should decode next authority set correctly")
		};

		let mmr_root_hash = signed_commitment
			.commitment
			.payload
			.get_decoded::<H256>(&MMR_ROOT_ID)
			.expect("Mmr root hash should decode correctly");

		let consensus_state = ConsensusState {
			mmr_root_hash,
			beefy_activation_block: inner.beefy_activation_block,
			latest_beefy_height: signed_commitment.commitment.block_number,
			current_authorities: current_authority_set.clone(),
			next_authorities: next_authority_set.clone(),
		};

		Ok(consensus_state)
	}
}

#[derive(Debug, Clone)]
pub struct ProverConsensusState {
	/// Inner consensus state tracked by the onchain light clients
	pub inner: ConsensusState,

	/// latest parachain height that has been finalized by BEEFY
	pub finalized_parachain_height: u64,
}

/// The BEEFY prover produces BEEFY consensus proofs using either the naive or zk variety. Consensus
/// proofs are produced when new messages are observed on the hyperbridge chain or when the
/// authority set changes.
pub struct BeefyProver<R: subxt::Config, P: subxt::Config> {
	consensus_state: ProverConsensusState,
	client: SubstrateClient<P>,
	prover: Prover<R, P>,
	minimum_finalization_height: u64,
}

impl<R, P> BeefyProver<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	P::Header: Send + Sync,
	<P::ExtrinsicParams as ExtrinsicParams<P::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<P, PlainTip>>,
	P::AccountId: From<sp_core::crypto::AccountId32> + Into<P::Address> + Clone + Send + Sync,
	P::Signature: From<MultiSignature> + Send + Sync,
{
	pub async fn new(
		config: &BeefyProverConfig,
		client: SubstrateClient<P>,
		consensus_state: ProverConsensusState,
		prover: Prover<R, P>,
	) -> Result<Self, anyhow::Error> {
		Ok(BeefyProver {
			consensus_state,
			prover,
			client,
			minimum_finalization_height: config.minimum_finalization_height,
		})
	}

	/// Runs the proving task. Will internally notify the appropriate channels of new epoch
	/// justifications as well as new proofs for ISMP messages.
	pub async fn run(&mut self) {
		let para_id = extract_para_id(self.client.state_machine_id().state_id)
			.expect("StateMachine should be either one of Polkadot or Kusama");

		loop {
			let future = async {
				loop {
					// tick the interval
					tokio::time::sleep(Duration::from_secs(10)).await;

					let (update, latest_height) = self.query_next_finalized_epoch().await?;
					self.consensus_state.inner.latest_beefy_height = latest_height as u32;

					if let Some((hash, set_id)) = update {
						// update should always be for the next set
						assert_eq!(set_id, self.consensus_state.inner.next_authorities.id);
						tracing::info!("Got update for next authority set: {set_id}");

						let header = self
							.prover
							.inner()
							.relay
							.rpc()
							.header(Some(hash))
							.await?
							.expect("Epoch change header exists");
						if let Some(commitment) =
							self.epoch_justification_for(header.number().into()).await?
						{
							tracing::info!(
								"Fetched justification: {:?} for next authority set {set_id}",
								commitment.commitment
							);
							let _proof = self
								.consensus_proof(
									commitment.clone(),
									self.consensus_state.inner.clone(),
								)
								.await?;

							// todo: put proof into mandatory queue on redis
							self.consensus_state.inner.latest_beefy_height =
								commitment.commitment.block_number;
							self.rotate_authorities(hash).await?;
							tracing::info!("New state {:#?}", self.consensus_state);
							// todo: serialize & push consensus state to redis
						}
					}

					let (latest_parachain_height, messages) =
						self.latest_ismp_message_events().await?;

					if messages.is_empty() {
						continue;
					}

					let lowest_message_height = messages
						.iter()
						.min_by(|a, b| a.meta.block_number.cmp(&b.meta.block_number))
						.expect("Messages is not empty; qed")
						.meta
						.block_number;

					let minimum_height = lowest_message_height + self.minimum_finalization_height;
					if minimum_height > latest_parachain_height {
						tracing::info!(
							"Waiting for {} blocks before proving finality for messages in the range: {lowest_message_height}..{latest_parachain_height}",
							minimum_height - latest_parachain_height
						);

						loop {
							tokio::time::sleep(Duration::from_secs(10)).await;
							let finalized_hash = self
								.prover
								.inner()
								.relay
								.rpc()
								.request::<R::Hash>("beefy_getFinalizedHead", rpc_params![])
								.await?;
							let header = query_parachain_header(
								&self.prover.inner().relay,
								finalized_hash,
								para_id,
							)
							.await?;

							if header.number as u64 >= minimum_height {
								break;
							}
						}
					}

					tracing::info!("Proving finality for messages in the range: {lowest_message_height}..{minimum_height}");
					let finalized_hash = self
						.prover
						.inner()
						.relay
						.rpc()
						.request::<R::Hash>("beefy_getFinalizedHead", rpc_params![])
						.await?;
					let (commitment, _) = fetch_latest_beefy_justification(
						&self.prover.inner().relay,
						finalized_hash,
					)
					.await?;
					let _proof = self
						.consensus_proof(commitment, self.consensus_state.inner.clone())
						.await?;
					// todo: put proof into mandatory queue
					self.consensus_state.finalized_parachain_height = latest_parachain_height;
					// todo: write consensus state to redis
				}

				#[allow(unreachable_code)]
				Ok::<_, anyhow::Error>(())
			};

			if let Err(err) = future.await {
				tracing::info!("Prover error: {err:?}")
			}
		}
	}

	/// Rotate the prover's known authority set, using the network view at the provided hash
	async fn rotate_authorities(&mut self, hash: R::Hash) -> Result<(), anyhow::Error> {
		self.consensus_state.inner.current_authorities =
			self.consensus_state.inner.next_authorities.clone();
		self.consensus_state.inner.next_authorities = {
			let key = runtime::storage().beefy_mmr_leaf().beefy_next_authorities();
			let next_authority_set = self
				.prover
				.inner()
				.relay
				.storage()
				.at(hash)
				.fetch(&key)
				.await?
				.expect("Should retrieve next authority set")
				.encode();
			BeefyNextAuthoritySet::decode(&mut &*next_authority_set)
				.expect("Should decode next authority set correctly")
		};

		Ok(())
	}

	/// Generate an encoded proof
	pub async fn consensus_proof(
		&self,
		signed_commitment: SignedCommitment<u32, Signature>,
		consensus_state: ConsensusState,
	) -> Result<Vec<u8>, anyhow::Error> {
		let encoded = match self.prover {
			Prover::Naive(ref naive) => {
				let message: BeefyConsensusProof =
					naive.consensus_proof(signed_commitment).await?.into();
				message.encode()
			},
			Prover::ZK(ref zk) => {
				let message = zk.consensus_proof(signed_commitment, consensus_state).await?;
				message.encode()
			},
		};

		Ok(encoded)
	}

	/// Returns the latest set of ismp messages that have been finalized and the latest finalized
	/// parachain block that was queried.
	pub async fn latest_ismp_message_events(
		&self,
	) -> Result<(u64, Vec<EventWithMetadata>), anyhow::Error> {
		let latest_height = self.consensus_state.finalized_parachain_height;
		let para_id = extract_para_id(self.client.state_machine_id().state_id)?;
		let finalized_hash = self
			.prover
			.inner()
			.relay
			.rpc()
			.request::<R::Hash>("beefy_getFinalizedHead", rpc_params![])
			.await?;

		let header =
			query_parachain_header(&self.prover.inner().relay, finalized_hash, para_id).await?;
		let finalized_height = header.number.into();
		if finalized_height <= latest_height {
			return Ok((latest_height, vec![]));
		}

		let events = self.query_ismp_events_with_metadata(latest_height, finalized_height).await?;

		let events = events
			.into_iter()
			.filter_map(|event| {
				if matches!(
					event.event,
					Event::PostRequest(_) |
						Event::PostResponse(_) | Event::PostRequestTimeoutHandled(_) |
						Event::PostResponseTimeoutHandled(_)
				) {
					return Some(event);
				}
				None
			})
			.collect::<Vec<_>>();

		return Ok((finalized_height, events));
	}

	/// Query the ismp events emitted within the block range (inclusive)
	async fn query_ismp_events_with_metadata(
		&self,
		previous_height: u64,
		latest_height: u64,
	) -> Result<Vec<EventWithMetadata>, anyhow::Error> {
		let range = (previous_height + 1)..=latest_height;

		if range.is_empty() {
			return Ok(Default::default());
		}

		let params = rpc_params![
			BlockNumberOrHash::<H256>::Number(previous_height.saturating_add(1) as u32),
			BlockNumberOrHash::<H256>::Number(latest_height as u32)
		];
		let response: HashMap<String, Vec<EventWithMetadata>> =
			self.client.client.rpc().request("ismp_queryEventsWithMetadata", params).await?;
		let events = response.into_values().flatten().collect();
		Ok(events)
	}

	/// Queries for any authority set changes in between the latest relay chain block finalized
	/// by beefy and the last known finalized block.
	pub async fn query_next_finalized_epoch(
		&self,
	) -> Result<(Option<(R::Hash, u64)>, u64), anyhow::Error> {
		let initial_height = self.consensus_state.inner.latest_beefy_height;
		let relay_client = self.prover.inner().relay.clone();
		let from = relay_client
			.rpc()
			.block_hash(Some(initial_height.into()))
			.await?
			.ok_or_else(|| anyhow!("Block hash should exist"))?;

		let finalized = self
			.prover
			.inner()
			.relay
			.rpc()
			.request::<R::Hash>("beefy_getFinalizedHead", rpc_params![])
			.await?;
		let to = relay_client
			.rpc()
			.header(Some(finalized))
			.await?
			.ok_or_else(|| anyhow!("Block hash should exist"))?;
		let changes = relay_client
			.rpc()
			.query_storage(vec![&VALIDATOR_SET_ID_KEY[..]], from, Some(finalized))
			.await?;

		let mut block_hash_and_set_id = changes
			.into_iter()
			.filter_map(|change| {
				change.changes[0]
					.clone()
					.1
					.and_then(|data| u64::decode(&mut &*data.0).ok())
					.map(|id| (change.block, id))
			})
			.filter(|(_, set_id)| *set_id >= self.consensus_state.inner.next_authorities.id);

		Ok((block_hash_and_set_id.next(), to.number().into()))
	}

	/// Performs a linear search for the BEEFY justification which finalizes the given epoch
	/// boundary
	pub async fn epoch_justification_for(
		&self,
		start: u64,
	) -> anyhow::Result<Option<SignedCommitment<u32, Signature>>> {
		let relay_client = self.prover.inner().relay.clone();

		for i in start..=(start + 50) {
			let hash = if let Some(hash) = relay_client.rpc().block_hash(Some(i.into())).await? {
				hash
			} else {
				continue;
			};

			if let Some(justifications) = relay_client
				.rpc()
				.block(Some(hash))
				.await?
				.ok_or_else(|| anyhow!("failed to find block for {hash:?}"))?
				.justifications
			{
				let beefy = justifications
					.into_iter()
					.find(|justfication| justfication.0 == sp_consensus_beefy::BEEFY_ENGINE_ID);

				if let Some((_, proof)) = beefy {
					let VersionedFinalityProof::V1(commitment) =
						VersionedFinalityProof::<u32, Signature>::decode(&mut &*proof)
							.expect("Beefy justification should decode correctly");
					return Ok(Some(commitment));
				}
			}
		}

		Ok(None)
	}
}

/// Query the parachain header that is finalized at the given relay chain block hash
pub async fn query_parachain_header<R: subxt::Config>(
	client: &OnlineClient<R>,
	hash: R::Hash,
	para_id: u32,
) -> Result<sp_runtime::generic::Header<u32, Keccak256>, anyhow::Error> {
	let head_data =
		client
			.storage()
			.at(hash)
			.fetch(&runtime::storage().paras().heads(
				&runtime::runtime_types::polkadot_parachain_primitives::primitives::Id(para_id),
			))
			.await?
			.ok_or_else(|| {
				anyhow!(
                "Could not fetch header for parachain with id {para_id} at block height {hash:?}"
            )
			})?;

	let header = sp_runtime::generic::Header::<u32, Keccak256>::decode(&mut &head_data.0[..])?;

	Ok(header)
}

#[cfg(test)]
mod tests {
	use super::*;
	use hex_literal::hex;
	use ismp::host::StateMachine;
	use redis_async::client::pubsub_connect;
	use rsmq_async::{Rsmq, RsmqConnection, RsmqError, RsmqOptions};
	use substrate_state_machine::HashAlgorithm;
	use tesseract_substrate::{
		config::{Blake2SubstrateChain, KeccakSubstrateChain},
		SubstrateConfig,
	};

	#[tokio::test]
	async fn test_can_produce_proofs() -> Result<(), anyhow::Error> {
		let substrate_config = SubstrateConfig {
			state_machine: StateMachine::Kusama(4009),
			hashing: Some(HashAlgorithm::Keccak),
			consensus_state_id: None,
			rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
			max_rpc_payload_size: None,
			signer: None,
			latest_height: None,
		};
		let substrate_client =
			SubstrateClient::<KeccakSubstrateChain>::new(substrate_config.clone()).await?;
		let relay_chain = subxt_utils::client::ws_client::<Blake2SubstrateChain>(
			"wss://hyperbridge-paseo-relay.blockops.network:443",
			u32::MAX,
		)
		.await?;
		let beefy_config = BeefyProverConfig { minimum_finalization_height: 25 };
		let prover_consensus_state = ProverConsensusState {
			inner: ConsensusState {
				latest_beefy_height: 0,
				beefy_activation_block: 0,
				mmr_root_hash: Default::default(),
				current_authorities: Default::default(),
				next_authorities: Default::default(),
			},
			finalized_parachain_height: 0,
		};
		let prover = Prover::new(ProverConfig {
			relay_rpc_ws: "wss://hyperbridge-paseo-relay.blockops.network:443".to_string(),
			para_rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
			para_ids: vec![4009],
			zk_beefy: None,
			max_rpc_payload_size: None,
		})
		.await?;
		let finalized_hash = prover
			.inner()
			.relay
			.rpc()
			.request::<H256>("beefy_getFinalizedHead", rpc_params![])
			.await?;
		let _ancient_hash =
			H256::from(hex!("6b33f31d9a5e46d0d735926a29e2293934db4acb785432af3184ede3107aa7b0"));
		let consensus_state = prover.query_initial_consensus_state(None).await?;

		let mut prover = BeefyProver::<Blake2SubstrateChain, _>::new(
			&beefy_config,
			substrate_client,
			prover_consensus_state,
			prover,
		)
		.await?;
		prover.consensus_state.inner = consensus_state;
		prover.consensus_state.finalized_parachain_height =
			query_parachain_header(&relay_chain, finalized_hash, 4009).await?.number as u64;

		prover.run().await;

		Ok(())
	}

	#[tokio::test]
	async fn test_redis_queues() -> Result<(), anyhow::Error> {
		let mut options = RsmqOptions::default();
		options.realtime = true;
		let mut rsmq = Rsmq::new(options).await.expect("connection failed");

		let queue = "myqueue2";
		if let Err(RsmqError::QueueExists) =
			rsmq.create_queue(queue, Some(Duration::ZERO), None, Some(-1)).await
		{
			println!("Queue already exists")
		}

		let pub_sub = pubsub_connect("localhost", 6379).await?;

		let mut stream = pub_sub.subscribe(&format!("rsmq:rt:{queue}")).await?;

		for i in 0..5 {
			rsmq.send_message(queue, format!("testmessage-{i}"), None)
				.await
				.expect("failed to send message");

			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		let mut count = 0;

		while let Some(value) = stream.next().await {
			println!("Got item from stream {value:?}");
			count += 1;
			if count == 5 {
				break
			}
		}

		count = 0;

		while let Some(message) = rsmq
			.receive_message::<String>(queue, Some(Duration::ZERO))
			.await
			.expect("cannot receive message")
		{
			count += 1;
			println!("Got: {message:?}, count: {count}");

			if count >= 10 {
				println!("Deleting {}", &message.id);
				rsmq.delete_message(queue, &message.id).await?;
			}
			// tokio::time::sleep(Duration::from_secs(1)).await;
		}

		println!("Ok done");

		// if let Some(message) = message {
		// }
		Ok(())
	}
}
