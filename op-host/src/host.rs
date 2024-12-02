use std::{sync::Arc, time::Duration};

use anyhow::anyhow;

use ethers::providers::Middleware;
use futures::StreamExt;
use geth_primitives::Header;
use ismp::{events::Event, messaging::CreateConsensusState};
use op_verifier::{calculate_output_root, CANNON};
use sp_core::{H160, H256, U256};
use tesseract_primitives::{Hasher, IsmpHost, IsmpProvider, StateMachineUpdated};

use crate::{
	abi::{dispute_game_factory::DisputeGameFactory, fault_dispute_game::FaultDisputeGame},
	OpHost,
};

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
		if let Some(dispute_game_factory_address) = self.dispute_game_factory.clone() {
			let (tx, recv) = tokio::sync::broadcast::channel(512);
			let client = self.clone();
			let initial_height = client.op_execution_client.get_block_number().await?.low_u64();
			// Watch for requests on the opstack chain
			// propose commitment after a confirmation delay
			tokio::task::spawn({
				let client = client.clone();
				async move {
					let mut latest_height = initial_height;
					loop {
						tokio::time::sleep(Duration::from_secs(30)).await;
						match lambda(&client, &mut latest_height, dispute_game_factory_address)
							.await
						{
							Ok(Some(proposal)) => match tx.send(proposal) {
								Ok(_) => {},
								Err(err) => {
									log::error!(
										"Failed to send state proposal over channel {err:?}"
									);
									return;
								},
							},
							Ok(_) => {},
							Err(e) => {
								log::error!("Encountered error fetching state proposal {e:?}");
							},
						}
					}
				}
			});

			let mut stream = tokio_stream::wrappers::BroadcastStream::new(recv);
			while let Some(proposal) = stream.next().await {
				match proposal {
					Ok(proposal) => {
						log::trace!(
							"Proposing state commitment for {:?}, block {:?}",
							self.provider.state_machine_id().state_id,
							proposal.block_number
						);
						let contract = DisputeGameFactory::new(
							dispute_game_factory_address,
							self.proposer.clone(),
						);

						let call = contract.create(
							proposal.game_type,
							proposal.root_claim.0,
							proposal.extra_data.into(),
						);
						let call = call.value(proposal.bond);

						let gas = call.estimate_gas().await;

						// Fetch gas price and use wait_for_success

						match gas {
							Ok(gas) => match call.gas(gas).send().await {
								// let evs = wait_for_success(client, progress, gas_price,
								// retry).await?;
								Ok(pending) => match pending.await {
									Ok(Some(receipt)) => {
										log::info!(
											"Tx submitted successfully {:?}",
											receipt.transaction_hash
										)
									},
									Ok(None) => {
										log::error!("Receipt not found for transaction");
									},
									Err(e) => {
										log::error!("Error waiting for receipt");
									},
								},
								Err(err) => {
									log::error!("Error broadcasting state proposal");
								},
							},
							Err(err) => {
								log::error!("Gas estimation for state proposal failed");
							},
						}
					},
					Err(e) => {
						log::error!("Stream returned error {e:?}")
					},
				}
			}
		} else {
			let mut stream = Box::pin(futures::stream::pending::<()>());
			while let Some(_) = stream.next().await {}
		};

		Err(anyhow!(
			"{}-{} consensus task has failed, Please restart relayer",
			self.provider().name(),
			counterparty.name()
		))
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		Ok(None)
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

async fn lambda(
	client: &OpHost,
	latest_height: &mut u64,
	dispute_game_factory_address: H160,
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
		// Wait for the chain to advance by a couple blocks
		let confirmation_delay = client.host.confirmation_delay.unwrap_or(200);
		log::trace!(
			"Waiting for {} blocks before proposing {:?} state commitment",
			confirmation_delay,
			client.provider.state_machine_id()
		);

		let proposal = loop {
			tokio::time::sleep(Duration::from_secs(30)).await;
			let l2_block_number = client.op_execution_client.get_block_number().await?;
			if l2_block_number.low_u64().saturating_sub(*latest_height) >= confirmation_delay {
				// Generate commitment for latest_block - (confirmation_delay / 4)
				let commitment_block_number = l2_block_number.low_u64() - (confirmation_delay / 4);
				let block = client
					.op_execution_client
					.get_block(commitment_block_number)
					.await?
					.ok_or_else(|| anyhow!("Failed to fetch block header"))?;

				let message_parser_proof = client
					.op_execution_client
					.get_proof(client.message_parser, vec![], Some(commitment_block_number.into()))
					.await?;

				let header = block.into();
				let l2_block_hash = Header::from(&header).hash::<Hasher>();
				let root_claim = calculate_output_root::<Hasher>(
					H256::zero(),
					header.state_root,
					message_parser_proof.storage_hash,
					l2_block_hash,
				);

				let extra_data = alloy_primitives::U256::from(l2_block_number.as_u64())
					.to_be_bytes::<32>()
					.to_vec();

				// Check that our commitment block is greater than the latest game
				let contract = DisputeGameFactory::new(
					dispute_game_factory_address,
					client.beacon_execution_client.clone(),
				);
				let latest_game_index = contract.game_count().call().await? - U256::one();
				let (_, _, dispute_proxy) =
					contract.game_at_index(latest_game_index).call().await?;
				let game =
					FaultDisputeGame::new(dispute_proxy, client.beacon_execution_client.clone());
				let latest_l2_block_number = game.l_2_block_number().call().await?.low_u64();
				// If the latest game block number is greater than our block, exit
				if latest_l2_block_number > commitment_block_number {
					break None;
				}

				let (proxy_addr, _) =
					contract.games(CANNON, root_claim.0, extra_data.clone().into()).call().await?;

				let bond = contract.init_bonds(CANNON).call().await?;
				// If game exists exit
				if proxy_addr != H160::zero() {
					break None;
				}

				*latest_height = commitment_block_number;

				break Some(StateProposal {
					root_claim,
					game_type: CANNON,
					block_number: commitment_block_number,
					extra_data,
					bond,
				});
			}
		};
		return Ok(proposal);
	}

	Ok(None)
}
