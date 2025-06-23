use std::{sync::Arc, time::Duration};

use anyhow::anyhow;

use ethers::{
	core::k256::ecdsa::SigningKey,
	middleware::SignerMiddleware,
	providers::{Http, Middleware, Provider},
	signers::Wallet,
};
use futures::StreamExt;
use geth_primitives::{new_u256, old_u256, CodecHeader, Header};
use ismp::{
	consensus::{StateCommitment, StateMachineId},
	events::Event,
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_optimism::{
	ConsensusState, OptimismConsensusProof, OptimismConsensusType, OptimismUpdate,
	OPTIMISM_CONSENSUS_CLIENT_ID,
};
use op_verifier::{calculate_output_root, CANNON, _PERMISSIONED};
use reqwest::Url;
use sp_core::{bytes::from_hex, Encode, H160, H256, U256};
use sync_committee_primitives::consensus_types::{BeaconBlockHeader, Checkpoint};
use sync_committee_prover::{
	responses::{self, finality_checkpoint_response::FinalityCheckpoint},
	routes::{finality_checkpoints, header_route},
};
use tesseract_evm::{
	gas_oracle::get_current_gas_cost_in_usd,
	tx::{get_chain_gas_limit, wait_for_success},
};
use tesseract_primitives::{Hasher, IsmpHost, IsmpProvider, StateMachineUpdated};

use crate::{
	abi::{dispute_game_factory::DisputeGameFactory, fault_dispute_game::FaultDisputeGame},
	OpHost, ProposerConfig,
};
use codec::Decode;

#[derive(Debug, Clone)]
pub struct StateProposal {
	/// output root
	pub root_claim: H256,
	/// l2 block number
	pub block_number: u64,
	/// Game type
	pub game_type: u32,
	/// Extra data
	pub extra_data: Vec<u8>,
	/// bond
	pub bond: U256,
}

#[async_trait::async_trait]
impl IsmpHost for OpHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		if self.dispute_game_factory.is_none() || self.host.proposer_config.is_none() {
			let mut stream = Box::pin(futures::stream::pending::<()>());
			while let Some(_) = stream.next().await {}
		} else {
			let dispute_game_factory_address = self
				.dispute_game_factory
				.clone()
				.ok_or_else(|| anyhow!("Expected dispute game factory address"))?;
			let proposer_config = self
				.host
				.proposer_config
				.clone()
				.ok_or_else(|| anyhow!("Expected proposer config"))?;
			let proposer = self.proposer.clone().ok_or_else(|| anyhow!("Expected proposer"))?;
			let (tx, recv) = tokio::sync::broadcast::channel(512);
			let client = self.clone();
			let initial_height = client.op_execution_client.get_block_number().await?.low_u64();
			// Watch for requests on the opstack chain
			// propose commitment after a confirmation delay
			tokio::task::spawn({
				let client = client.clone();
				let dispute_game_factory_address = dispute_game_factory_address.clone();
				let proposer_config = proposer_config.clone();
				async move {
					let mut latest_height = initial_height;
					log::trace!(target: "tesseract", "Started Proposer for {:?} at {latest_height}", client.evm.state_machine());

					loop {
						tokio::time::sleep(Duration::from_secs(30)).await;
						match construct_state_proposal(
							&client,
							&mut latest_height,
							dispute_game_factory_address,
							&proposer_config,
						)
						.await
						{
							Ok(Some(proposal)) => match tx.send(proposal) {
								Ok(_) => {},
								Err(err) => {
									log::error!(target: "tesseract",
										"Failed to send state proposal over channel {err:?}"
									);
									return;
								},
							},
							Ok(_) => {},
							Err(e) => {
								log::error!(target: "tesseract","Encountered error fetching state proposal {e:?}");
							},
						}
					}
				}
			});

			let mut stream = tokio_stream::wrappers::BroadcastStream::new(recv);
			while let Some(proposal) = stream.next().await {
				match proposal {
					Ok(proposal) => {
						if let Err(err) = submit_state_proposal(
							&self,
							dispute_game_factory_address,
							proposer.clone(),
							&proposer_config,
							proposal,
						)
						.await
						{
							log::error!(target: "tesseract", "Error submitting state proposal {err:?}")
						}
					},
					Err(e) => {
						log::error!(target: "tesseract","Stream returned error {e:?}")
					},
				}
			}
		}

		submit_consensus_update(self, counterparty.clone()).await?;

		Err(anyhow!(
			"{}-{} consensus task has failed, Please restart relayer",
			self.provider().name(),
			counterparty.name()
		))
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let mut state_machine_commitments = vec![];

		let number = self.op_execution_client.get_block_number().await?;
		let block = self.op_execution_client.get_block(number).await?.ok_or_else(|| {
			anyhow!("Didn't find block with number {number} on {:?}", self.evm.state_machine)
		})?;
		let state_machine_id = StateMachineId {
			state_id: self.state_machine,
			consensus_state_id: self.consensus_state_id.clone(),
		};
		let initial_consensus_state = ConsensusState {
			finalized_height: number.as_u64(),
			state_machine_id,
			l1_state_machine_id: StateMachineId {
				state_id: self.l1_state_machine,
				consensus_state_id: self.l1_consensus_state_id,
			},
			state_root: block.state_root.0.into(),
			optimism_consensus_type: None,
			respected_game_types: Some(vec![CANNON, _PERMISSIONED]),
		};

		state_machine_commitments.push((
			state_machine_id,
			StateCommitmentHeight {
				commitment: StateCommitment {
					timestamp: block.timestamp.as_u64(),
					overlay_root: None,
					state_root: block.state_root.0.into(),
				},
				height: number.as_u64(),
			},
		));
		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: OPTIMISM_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: state_machine_commitments
				.iter()
				.map(|(state_machine, ..)| (state_machine.state_id, 5 * 60))
				.collect(),
			state_machine_commitments,
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

async fn construct_state_proposal(
	client: &OpHost,
	latest_height: &mut u64,
	dispute_game_factory_address: H160,
	proposer_config: &ProposerConfig,
) -> Result<Option<StateProposal>, anyhow::Error> {
	let block_number = client.op_execution_client.get_block_number().await?.as_u64();
	if block_number <= *latest_height {
		return Ok::<_, anyhow::Error>(None);
	}

	let event = StateMachineUpdated {
		state_machine_id: client.provider.state_machine_id(),
		latest_height: block_number,
	};
	let events = client.provider.query_ismp_events(*latest_height, event).await?;
	*latest_height = block_number;
	let event = events.into_iter().find(|ev| match &ev {
		Event::PostRequest(_) | Event::GetRequest(_) | Event::PostResponse(_) => true,
		_ => false,
	});

	if event.is_some() {
		// Wait for end of current l1 epoch
		let l2_header = client
			.op_execution_client
			.get_block(*latest_height)
			.await?
			.ok_or_else(|| anyhow!(" Block should exist"))?;
		let l2_header: CodecHeader = l2_header.into();
		let parent_beacon_root = l2_header
			.parent_beacon_root
			.ok_or_else(|| anyhow!("Parent beacon root should be present"))?;
		let beacon_block_id = get_block_id(parent_beacon_root);
		let beacon_header = fetch_beacon_header(client, proposer_config, &beacon_block_id).await?;
		let parent_beacon_epoch = beacon_header.slot / 32;
		log::trace!(target: "tesseract",
			"{} Proposer: waiting until parent beacon block is finalized before proposing;  beacon block header -> {:?}",
			client.provider.state_machine_id().state_id,
			parent_beacon_root
		);

		struct LatestGameData {
			l2_block_number: u64,
			game: FaultDisputeGame<Provider<Http>>,
		}

		let proposal = loop {
			// We can only propose a state commitment when it is derived from a finalized beacon
			// block
			let finalized_epoch =
				fetch_finalized_checkpoint(client, proposer_config, "head").await?.epoch;
			if finalized_epoch >= parent_beacon_epoch {
				log::trace!(target: "tesseract", "Constructing state proposal for {:?} at block {:?}", client.provider.state_machine_id().state_id, latest_height);
				// refetch the l2 header incase there has been a reorg
				let l2_header = client
					.op_execution_client
					.get_block(*latest_height)
					.await?
					.ok_or_else(|| anyhow!(" Block should exist"))?;
				let l2_header = l2_header.into();
				let l2_block_hash = Header::from(&l2_header).hash::<Hasher>();
				let commitment_block_number = *latest_height;

				let message_parser_proof = client
					.op_execution_client
					.get_proof(
						ethers::types::H160(client.message_parser.0),
						vec![],
						Some(commitment_block_number.into()),
					)
					.await?;

				let root_claim = calculate_output_root::<Hasher>(
					H256::zero(),
					l2_header.state_root,
					message_parser_proof.storage_hash.0.into(),
					l2_block_hash,
				);

				let extra_data = alloy_primitives::U256::from(commitment_block_number)
					.to_be_bytes::<32>()
					.to_vec();

				let respected_game_type = CANNON;

				// Check that our commitment block is greater than the latest game
				let contract = DisputeGameFactory::new(
					dispute_game_factory_address.0,
					client.beacon_execution_client.clone(),
				);
				// Find the latest valid root claim with the respected game type,
				// We only yield a new state proposal if
				// 1. The most recent 3 games are invalid
				// 2. The latest valid game is for a block less than our commitment block number
				// 3. The op-proposer interval for proposing is not yet in its last quarter
				let latest_game_index = contract.game_count().call().await? - old_u256(U256::one());
				let mut proposal = None;
				let mut latest_valid_game = None;
				// We would inspect the first five most recent games
				let range =
					(latest_game_index.low_u64().saturating_sub(2))..=latest_game_index.low_u64();
				for game_index in range.rev() {
					let (_, _, dispute_proxy) =
						contract.game_at_index(game_index.into()).call().await?;

					let game = FaultDisputeGame::new(
						dispute_proxy,
						client.beacon_execution_client.clone(),
					);

					let latest_game_type = game.game_type().await?;
					// If this game is not the respected game type we continue our search
					if latest_game_type != respected_game_type {
						continue;
					}

					let latest_claim = game.root_claim().call().await?;
					let latest_claim_l2_block_number =
						game.l_2_block_number().call().await?.low_u64();

					let latest_claim_header = client
						.op_execution_client
						.get_block(latest_claim_l2_block_number)
						.await?
						.ok_or_else(|| anyhow!(" Block should exist"))?;
					let latest_claim_header = latest_claim_header.into();
					let latest_claim_message_parser_proof = client
						.op_execution_client
						.get_proof(
							ethers::types::H160(client.message_parser.0),
							vec![],
							Some(latest_claim_l2_block_number.into()),
						)
						.await?;
					let latest_claim_header_block_hash =
						Header::from(&latest_claim_header).hash::<Hasher>();

					let calculated_latest_root_claim = calculate_output_root::<Hasher>(
						H256::zero(),
						latest_claim_header.state_root,
						latest_claim_message_parser_proof.storage_hash.0.into(),
						latest_claim_header_block_hash,
					);

					// If the claim in the game is incorrect we continue
					if calculated_latest_root_claim.0 != latest_claim {
						continue;
					}

					latest_valid_game = Some(LatestGameData {
						game,
						l2_block_number: latest_claim_l2_block_number,
					});

					break;
				}

				if let Some(latest_valid_game) = latest_valid_game {
					// If the latest game block number is greater than our block and its root claim
					// is correct exit
					if latest_valid_game.l2_block_number > commitment_block_number {
						log::trace!(target: "tesseract","Latest proposed block {} > commitment block{commitment_block_number}", latest_valid_game.l2_block_number);
						break proposal;
					}

					let (proxy_addr, _) = contract
						.games(respected_game_type, root_claim.0, extra_data.clone().into())
						.call()
						.await?;

					// If game exists exit
					if proxy_addr.0 != H160::zero().0 {
						log::trace!(target: "tesseract","State commitment for {commitment_block_number} has already been proposed");
						break proposal;
					}

					// When was the last claim submitted
					let creation_time = latest_valid_game.game.created_at().call().await?;
					let current_block_num =
						client.beacon_execution_client.get_block_number().await?;
					let current_block_header = client
						.beacon_execution_client
						.get_block(current_block_num.as_u64())
						.await?
						.ok_or_else(|| anyhow!("Failed to fetch latest L1 header"))?;
					let diff =
						current_block_header.timestamp.low_u64().saturating_sub(creation_time);

					let creator = latest_valid_game.game.game_creator().call().await?;
					let op_proposer = from_hex(&proposer_config.op_proposer)?;

					// If the time since the last proposal is greater than 3/4 of the proposal
					// interval then it doesn't make economic sense to continue with this
					// proposal
					if creator.0.to_vec() == op_proposer &&
						diff >= (3 * proposer_config.proposer_interval / 4)
					{
						log::trace!(target: "tesseract","Skipping proposal for {commitment_block_number}, Official op-proposer should be making a proposal in {} seconds",
							proposer_config.proposer_interval.saturating_sub((3 * proposer_config.proposer_interval )/ 4));
						break proposal;
					}

					let bond = contract.init_bonds(respected_game_type).call().await?;

					proposal = Some(StateProposal {
						root_claim,
						game_type: respected_game_type,
						block_number: commitment_block_number,
						extra_data: extra_data.clone(),
						bond: new_u256(bond),
					});

					break proposal;
				} else {
					log::trace!(target: "tesseract","Recent games are invalid, moving ahead with proposal for {commitment_block_number}");
					let bond = contract.init_bonds(respected_game_type).call().await?;
					proposal = Some(StateProposal {
						root_claim,
						game_type: respected_game_type,
						block_number: commitment_block_number,
						extra_data: extra_data.clone(),
						bond: new_u256(bond),
					});

					break proposal;
				}
			}

			tokio::time::sleep(Duration::from_secs(30)).await;
		};
		return Ok(proposal);
	}
	Ok(None)
}

async fn submit_state_proposal(
	client: &OpHost,
	dispute_game_factory_address: H160,
	proposer: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
	proposer_config: &ProposerConfig,
	proposal: StateProposal,
) -> Result<(), anyhow::Error> {
	log::trace!(target: "tesseract",
		"Proposing state commitment for {:?}, block {:?}",
		client.provider.state_machine_id().state_id,
		proposal.block_number
	);
	let contract = DisputeGameFactory::new(dispute_game_factory_address.0, proposer.clone());

	let call =
		contract.create(proposal.game_type, proposal.root_claim.0, proposal.extra_data.into());
	let call = call.value(old_u256(proposal.bond));

	let gas_limit = call
		.estimate_gas()
		.await
		.unwrap_or(get_chain_gas_limit(client.l1_state_machine).into());

	// Fetch L1 gas price
	let gas_breakdown = get_current_gas_cost_in_usd(
		client.l1_state_machine,
		&proposer_config.l1_etherscan_api_key,
		client.beacon_execution_client.clone(),
	)
	.await?;

	let call = call.gas_price(old_u256(gas_breakdown.gas_price)).gas(gas_limit);

	let tx = call.send().await?;
	wait_for_success(
		&client.l1_state_machine,
		&proposer_config.l1_etherscan_api_key,
		client.beacon_execution_client.clone(),
		proposer.clone(),
		tx,
		Some(gas_breakdown.gas_price),
		Some(call.clone().gas(gas_limit)),
		true,
	)
	.await?;

	Ok(())
}

async fn fetch_beacon_header(
	client: &OpHost,
	proposer_config: &ProposerConfig,
	block_id: &str,
) -> Result<BeaconBlockHeader, anyhow::Error> {
	let beacon_consensus_client = client
		.beacon_consensus_client
		.clone()
		.expect("Expected consensus client to be available");
	let primary_url = proposer_config
		.beacon_consensus_rpcs
		.get(0)
		.cloned()
		.ok_or_else(|| anyhow!("Missing beacon rpc urls"))?;
	let path = header_route(block_id);
	let full_url = Url::parse(&format!("{}{}", primary_url, path))?;
	let response = beacon_consensus_client
		.get(full_url)
		.send()
		.await
		.map_err(|e| anyhow!("Failed to fetch header with id {block_id} due to error {e:?}"))?;

	let response_data = response
		.json::<responses::beacon_block_header_response::Response>()
		.await
		.map_err(|e| anyhow!("Failed to fetch header with id {block_id} due to error {e:?}"))?;

	let beacon_block_header = response_data.data.header.message;

	Ok(beacon_block_header)
}

async fn fetch_finalized_checkpoint(
	client: &OpHost,
	proposer_config: &ProposerConfig,
	block_id: &str,
) -> Result<Checkpoint, anyhow::Error> {
	let beacon_consensus_client = client
		.beacon_consensus_client
		.clone()
		.expect("Expected consensus client to be available");
	let primary_url = proposer_config
		.beacon_consensus_rpcs
		.get(0)
		.cloned()
		.ok_or_else(|| anyhow!("Missing beacon rpc urls"))?;
	let path = finality_checkpoints(block_id);
	let full_url = Url::parse(&format!("{}{}", primary_url, path))?;
	let response = beacon_consensus_client
		.get(full_url)
		.send()
		.await
		.map_err(|e| anyhow!("Failed to fetch header with id {block_id} due to error {e:?}"))?;

	#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
	struct CheckpointResponse {
		execution_optimistic: bool,
		data: FinalityCheckpoint,
	}
	let response_data = response
		.json::<CheckpointResponse>()
		.await
		.map_err(|e| anyhow!("Failed to fetch header with id {block_id} due to error {e:?}"))?;

	let checkpoint = response_data.data.finalized;

	Ok(checkpoint)
}

async fn submit_consensus_update(
	client: &OpHost,
	counterparty: Arc<dyn IsmpProvider>,
) -> Result<(), anyhow::Error> {
	let consensus_state =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;

	let l1_state_machine_id = StateMachineId {
		state_id: client.l1_state_machine,
		consensus_state_id: client.l1_consensus_state_id,
	};
	let mut stream = counterparty
		.state_machine_update_notification(l1_state_machine_id.clone())
		.await?;
	let mut latest_height = counterparty.query_latest_height(l1_state_machine_id).await? as u64;
	while let Some(res) = stream.next().await {
		match res {
			Ok(event) => match consensus_state.optimism_consensus_type {
				Some(OptimismConsensusType::OpL2Oracle) => {
					let event_height = event.latest_height;
					let latest_event = client.latest_event(latest_height, event_height).await?;

					if let Some(event) = latest_event {
						let payload = client.fetch_op_payload(event_height, event).await?;
						let update = OptimismUpdate {
							state_machine_id: StateMachineId {
								state_id: client.state_machine,
								consensus_state_id: client.consensus_state_id,
							},
							l1_height: event_height,
							proof: OptimismConsensusProof::OpL2Oracle(payload),
						};

						let consensus_message = ConsensusMessage {
							consensus_proof: update.encode(),
							consensus_state_id: client.consensus_state_id,
							signer: counterparty.address(),
						};

						let _ = counterparty
							.submit(
								vec![Message::Consensus(consensus_message)],
								counterparty.state_machine_id().state_id,
							)
							.await;

						latest_height = event_height;
					}
				},
				Some(OptimismConsensusType::OpFaultProofGames) => {
					let event_height = event.latest_height;

					if let Some(respected_game_types) = consensus_state.respected_game_types.clone()
					{
						let latest_event = client
							.latest_dispute_games(
								latest_height,
								event_height,
								respected_game_types.clone(),
							)
							.await?;

						let maybe_payload = client
							.fetch_dispute_game_payload(
								event_height,
								respected_game_types,
								latest_event,
							)
							.await?;

						if let Some(payload) = maybe_payload {
							let update = OptimismUpdate {
								state_machine_id: StateMachineId {
									state_id: client.state_machine,
									consensus_state_id: client.consensus_state_id,
								},
								l1_height: event_height,
								proof: OptimismConsensusProof::OpFaultProofGames(payload),
							};

							let consensus_message = ConsensusMessage {
								consensus_proof: update.encode(),
								consensus_state_id: client.consensus_state_id,
								signer: counterparty.address(),
							};

							let _ = counterparty
								.submit(
									vec![Message::Consensus(consensus_message)],
									counterparty.state_machine_id().state_id,
								)
								.await;
						}
					}
					latest_height = event_height;
				},
				_ => {},
			},
			Err(err) => {
				log::error!("State machine update stream returned an error {err:?}")
			},
		}
	}

	Ok(())
}

fn get_block_id(root: H256) -> String {
	let mut block_id = ethers::utils::hex::encode(root.0.to_vec());
	block_id.insert_str(0, "0x");
	block_id
}
