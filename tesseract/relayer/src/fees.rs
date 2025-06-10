use crate::{config::HyperbridgeConfig, create_client_map, logging};
use anyhow::anyhow;
use ethers::providers::interval;
use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, Message, Proof, RequestMessage},
	router::Request,
};
use sp_core::U256;
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tesseract_primitives::{
	config::RelayerConfig, observe_challenge_period, wait_for_state_machine_update, Cost, Hasher,
	HyperbridgeClaim, IsmpProvider, Query, WithdrawFundsResult,
};
use tesseract_substrate::config::KeccakSubstrateChain;
use tracing::instrument;
use transaction_fees::TransactionPayment;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Withdraw fees on hyperbridge
	AccumulateFees(AccumulateFees),
}

#[derive(Debug, clap::Parser)]
#[command(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
pub struct AccumulateFees {
	/// Accumulate fees and withdraw the funds
	#[arg(short, long)]
	pub withdraw: bool,
	/// Gas limit for executing withdrawal requests on both chains
	#[arg(short, long)]
	pub gas_limit: Option<u64>,
	/// Wait for all deliveries or skip unavailable ones
	#[arg(short, long)]
	pub wait: bool,
}

impl AccumulateFees {
	/// Accumulate fees accrued through deliveries from source to dest and dest to source

	pub async fn accumulate_fees(&self, config_path: String, db: String) -> anyhow::Result<()> {
		logging::animated_logs()?;
		let config = HyperbridgeConfig::parse_conf(&config_path).await?;
		let HyperbridgeConfig { hyperbridge: hyperbridge_config, .. } = config.clone();

		let hyperbridge = tesseract_substrate::SubstrateClient::<KeccakSubstrateChain>::new(
			hyperbridge_config.clone(),
		)
		.await?;

		let clients = create_client_map(config, Arc::new(hyperbridge.clone())).await?;

		// early return if withdrawing
		if self.withdraw {
			let tx_payment = TransactionPayment::initialize(&db).await?;
			self.withdraw(tx_payment, &hyperbridge, clients).await?;
			return Ok(());
		}

		let tx_payment = TransactionPayment::initialize(&db).await?;
		log::info!("Initialized database");
		let stream = futures::stream::iter(tx_payment.distinct_deliveries().await?.into_iter());
		stream.for_each_concurrent(None, |delivery| {
			let source_chain = StateMachine::from_str(&delivery.source_chain)
				.expect("Invalid Source State Machine provided");
			let dest_chain = StateMachine::from_str(&delivery.dest_chain)
				.expect("Invalid Dest State Machine provided");
			let source = clients
				.get(&source_chain)
				.expect(&format!("Client not found for {source_chain:?}")).clone();
			let dest = clients
				.get(&dest_chain)
				.expect(&format!("Client not found for {dest_chain:?}")).clone();
			let tx_payment = tx_payment.clone();
			let hyperbridge = hyperbridge.clone();
			async move {

				let lambda = || async {
					let source_height = hyperbridge.query_latest_height(source.state_machine_id()).await?;
					let dest_height = hyperbridge.query_latest_height(dest.state_machine_id()).await?;

					let highest_delivery_height_to_dest = tx_payment
						.highest_delivery_height(
							source.state_machine_id().state_id,
							dest.state_machine_id().state_id,
						)
						.await?;
					let highest_delivery_height_to_source = tx_payment
						.highest_delivery_height(
							dest.state_machine_id().state_id,
							source.state_machine_id().state_id,
						)
						.await?;
					// If no messages have been delivered we skip pair
					if highest_delivery_height_to_dest.is_none() &&
						highest_delivery_height_to_source.is_none()
					{
						log::info!("No deliveries found in db for {source_chain}->{dest_chain}");
						return Ok::<_, anyhow::Error>(())
					}

					if let Some(height) = highest_delivery_height_to_dest {
						let height = if height > dest_height.into() && self.wait {
							let height = wait_for_state_machine_update(
								dest.state_machine_id(),
								Arc::new(hyperbridge.clone()),
								dest.clone(),
								height,
							)
								.await?;
							Some(height)
						} else if height <= dest_height.into() {
							Some(dest_height.into())
						} else {
							None
						};

						match height {
							Some(height) => {
								// Create claim proof for deliveries from source to dest
								log::info!("Creating withdrawal proof from db for deliveries from {source_chain}->{dest_chain}");
								let proofs = tx_payment
									.create_claim_proof(source_height.into(), height, source.clone(), dest.clone(), &hyperbridge)
									.await?;

								if proofs.is_empty() {
									log::info!("All fees in the database for  {source_chain}->{dest_chain} have been successfully accumulated in a previous attempt")
								} else {
									observe_challenge_period(dest.clone(), Arc::new(hyperbridge.clone()), height).await?;
								}

								log::info!(
									"Submitting proofs for {source_chain}->{dest_chain} to hyperbridge"
								);
								for proof in proofs {
									hyperbridge.accumulate_fees(proof.clone()).await?;
									// Don't panic if delete operation failed
									match tx_payment.delete_claimed_entries(proof.commitments).await {
										Err(_) => {
										log::error!("An Error occured while deleting claimed fees from the db, the claimed keys will be deleted in the next fee accumulation attempt");
										}
										_ => {}
									};
								}

								log::info!("Proofs sucessfully submitted");
							},
							None => {
								log::info!("Skipping fee accumulation for {source_chain}->{dest_chain}: state machine update not yet available on hyperbridge");
							},
						}
					}

					if let Some(height) = highest_delivery_height_to_source {
						let height = if height > source_height.into() && self.wait {
							let height = wait_for_state_machine_update(
								source.state_machine_id(),
								Arc::new(hyperbridge.clone()),
								source.clone(),
								height,
							)
								.await?;
							Some(height)
						} else if height <= source_height.into() {
							Some(source_height.into())
						} else {
							None
						};

						match height {
							Some(height) => {
								// Create claim proof for deliveries from dest to source
								log::info!("Creating withdrawal proof from db for deliveries from {dest_chain}->{source_chain}");
								let proofs = tx_payment
									.create_claim_proof(dest_height.into(), height, dest.clone(), source.clone(), &hyperbridge)
									.await?;

								if proofs.is_empty() {
									log::info!("All fees in the database for  {dest_chain}->{source_chain} have been successfully accumulated in a previous attempt")
								}
								else {
									observe_challenge_period(source.clone(), Arc::new(hyperbridge.clone()), height).await?;
								}
								log::info!(
									"Submitting proofs for {dest_chain}->{source_chain} to hyperbridge"
								);
								for proof in proofs {
									hyperbridge.accumulate_fees(proof.clone()).await?;
									// Don't panic if delete operation failed, it will be retried in another fee accumulation attempt
									match tx_payment.delete_claimed_entries(proof.commitments).await {
										Err(_) => {
											log::error!("An Error occured while deleting claimed fees from the db, the claimed keys will be deleted in the next fee accumulation attempt");
										}
										_ => {}
									}
								}
								log::info!("Proof sucessfully submitted");
							},
							None => {
								log::info!("Skipping fee accumulation for {dest_chain}->{source_chain}: state machine update not yet available on hyperbridge");
							},
						}
					}

					Ok(())
				};
				match lambda().await {
					Ok(_) => {},
					Err(e) => log::error!("Fee accumulation for {dest_chain}->{source_chain} failed: {e:?}"),
				}
			}

		}).await;

		Ok(())
	}

	pub async fn withdraw<C: IsmpProvider + HyperbridgeClaim + Clone>(
		&self,
		tx: TransactionPayment,
		hyperbridge: &C,
		clients: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	) -> anyhow::Result<()> {
		let stream = futures::stream::iter(clients.keys().cloned().into_iter());

		stream
			.for_each_concurrent(None, |chain| {
				let client =
					clients.get(&chain).expect(&format!("Client not found for {chain}")).clone();
				let hyperbridge = hyperbridge.clone();
				let tx = tx.clone();
				async move {
					let lambda = || async {
						// lets try to deliver any pending requests in the db
						let (pending_withdrawals, ids): (Vec<_>, Vec<_>) = tx.pending_withdrawals(&chain).await?.into_iter().unzip();
						for pending in pending_withdrawals {
							deliver_post_request(client.clone(), &hyperbridge, pending).await?;
						}
						// can this fail?
						if let Err(e) = tx.delete_pending_withdrawals(ids).await {
							tracing::error!("Error encountered while deleting pending withdrawals from the db: {e:?}, \n NOTE: The withdrawal request was successfully delivered.");
						}

						let amount = hyperbridge.available_amount(client.clone(), &chain).await?;

						let fee_token_decimals = client.fee_token_decimals().await?;
						if amount < U256::from(10u128 * 10u128.pow(fee_token_decimals.into())) {
							log::info!("Unclaimed balance on {chain} is less than $10, exiting");
							return Ok::<_, anyhow::Error>(());
						}

						log::info!(
							"Submitting withdrawal request to {chain}  for amount ${}",
							amount / U256::from(10u128.pow(fee_token_decimals.into()))
						);
						let result = hyperbridge
							.withdraw_funds(client.clone(), chain)
							.await?;
						log::info!("Request submitted to hyperbridge successfully");
						log::info!("Starting delivery of withdrawal message to {}", chain);
						// Wait for state machine update
						// persist the withdrawal in-case delivery fails, so it's not lost forever
						let ids = tx.store_pending_withdrawals(vec![result.clone()]).await?;

						match deliver_post_request(client.clone(), &hyperbridge, result.clone()).await {
							Ok(_) => {
								if let Err(e) = tx.delete_pending_withdrawals(ids).await {
									tracing::error!("Error encountered while deleting pending withdrawals from the db: {e:?}, \n NOTE: The withdrawal request was successfully delivered.");
								}
							},
							Err(err) => {
								tracing::info!("Failed to deliver withdrawal request: {err:?}, they will be retried.");
							}
						};
						Ok(())
					};

					match lambda().await {
						Ok(_) => {},
						Err(e) => log::error!("Failed to complete a withdrawal request: {e:?}"),
					}
				}
			})
			.await;

		Ok(())
	}
}

/// For every configured `withdrawal_frequency`, will attempt to withdraw all unclaimed fees on
/// hyperbridge.
pub async fn auto_withdraw<C>(
	hyperbridge: C,
	clients: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	config: RelayerConfig,
	db: Arc<TransactionPayment>,
) -> anyhow::Result<()>
where
	C: IsmpProvider + HyperbridgeClaim + Clone,
{
	// default to 1 day
	let frequency = Duration::from_secs(config.withdrawal_frequency.unwrap_or(86_400));
	tracing::info!("Auto-withdraw frequency set to {:?}", frequency);
	let min_amount: U256 = (config
		.minimum_withdrawal_amount
		.map(|val| std::cmp::max(val, 10))
		.unwrap_or(100) as u128 *
		10u128.pow(18))
	.into();
	tracing::info!("Minimum auto-withdrawal amount set to ${:?}", Cost(min_amount));
	let mut interval = interval(frequency);

	while let Some(_) = interval.next().await {
		let stream = futures::stream::iter(clients.keys().cloned().into_iter());
		stream
			.for_each_concurrent(None, |chain| {
				let client =
					clients.get(&chain).expect(&format!("Client not found for {chain}")).clone();
				let hyperbridge = hyperbridge.clone();
				let moved_db = db.clone();
				async move {
					let lambda = || async {
						// lets try to deliver any pending requests in the db
						let (pending_withdrawals, ids): (Vec<_>, Vec<_>) = moved_db.pending_withdrawals(&chain).await?.into_iter().unzip();
						for pending in pending_withdrawals {
							deliver_post_request(client.clone(), &hyperbridge, pending).await?;
						}
						// can this fail?
						if let Err(e) = moved_db.delete_pending_withdrawals(ids).await {
							tracing::error!("Error encountered while deleting pending withdrawals from the db: {e:?}, \n NOTE: The withdrawal request was successfully delivered.");
						}

						let amount = hyperbridge.available_amount(client.clone(), &chain).await?;
						let fee_token_decimals = client.fee_token_decimals().await?;
						// default to $100
						let min_amount: U256 = (config
							.minimum_withdrawal_amount
							.map(|val| std::cmp::max(val, 10))
							.unwrap_or(100) as u128
							* 10u128.pow(fee_token_decimals.into()))
						.into();
						if amount < min_amount {
							tracing::info!("Unclaimed balance {amount} on {chain} is < minimum_withdrawal_amount: {min_amount}, exiting");
							return Ok::<_, anyhow::Error>(());
						}

						tracing::info!(
							"Submitting withdrawal request to hyperbridge for amount ${} on {chain}",
							amount / U256::from(10u128.pow(fee_token_decimals.into()))
						);
						let result = hyperbridge
							.withdraw_funds(client.clone(), chain)
							.await?;
						tracing::info!("Request submitted to hyperbridge successfully");
						tracing::info!("Starting delivery of withdrawal message to {}", chain);

						// persist the withdrawal in-case delivery fails, so it's not lost forever
						let ids = moved_db.store_pending_withdrawals(vec![result.clone()]).await?;

						match deliver_post_request(client.clone(), &hyperbridge, result.clone()).await {
							Ok(_) => {
								if let Err(e) = moved_db.delete_pending_withdrawals(ids).await {
									tracing::error!("Error encountered while deleting pending withdrawals from the db: {e:?}, \n NOTE: The withdrawal request was successfully delivered.");
								}
							},
							Err(err) => {
								tracing::info!("Failed to deliver withdrawal request: {err:?}, they will be retried.");
							}
						};
						Ok(())
					};

					match lambda().await {
						Ok(_) => {},
						Err(e) => log::error!("Failed to complete an auto-withdrawal: {e:?}"),
					}
				}
			})
			.await;
	}

	Ok(())
}

#[instrument(name = "Delivering post request to ", skip_all, fields(destination = dest_chain.state_machine_id().state_id.to_string()))]
async fn deliver_post_request<D: IsmpProvider>(
	dest_chain: Arc<dyn IsmpProvider>,
	hyperbridge: &D,
	result: WithdrawFundsResult,
) -> anyhow::Result<()> {
	let mut latest_height =
		dest_chain.query_latest_height(hyperbridge.state_machine_id()).await? as u64;

	if result.block > latest_height {
		// then we have to wait
		log::info!(
			"Waiting for state machine update that finalizes withdraw height: {}",
			result.block
		);
		let mut stream = dest_chain
			.state_machine_update_notification(hyperbridge.state_machine_id())
			.await?;

		latest_height = loop {
			match stream.next().await {
				Some(Ok(event)) =>
					if event.latest_height < result.block {
						continue;
					} else {
						log::info!("Found a state machine update: {}", event.latest_height);
						break event.latest_height;
					},
				Some(Err(_)) => {
					log::error!(
						"An error occured waiting for state machine update from {}, Retrying",
						dest_chain.name()
					);
				},
				None => Err(anyhow!("Error waiting for state machine update"))?,
			}
		};
	}

	let query = Query {
		source_chain: result.post.source,
		dest_chain: result.post.dest,
		nonce: result.post.nonce,
		commitment: hash_request::<Hasher>(&Request::Post(result.post.clone())),
	};
	log::info!("Querying request proof from hyperbridge at {}", latest_height);
	let proof = hyperbridge
		.query_requests_proof(latest_height, vec![query], dest_chain.state_machine_id().state_id)
		.await?;
	log::info!("Successfully queried request proof from hyperbridge");
	let msg = RequestMessage {
		requests: vec![result.post.clone()],
		proof: Proof {
			height: StateMachineHeight {
				id: hyperbridge.state_machine_id(),
				height: latest_height,
			},
			proof,
		},
		signer: dest_chain.address(),
	};

	log::info!("Submitting post request to {}", dest_chain.state_machine_id().state_id);

	let mut count = 5;
	while count != 0 {
		if let Err(e) = dest_chain
			.submit(vec![Message::Request(msg.clone())], hyperbridge.state_machine_id().state_id)
			.await
		{
			log::info!(
					"Encountered error trying to submit withdrawal request to {}.\n{e:?}\nWill retry {count} more times.",
					dest_chain.state_machine_id().state_id
				);
			count -= 1;
		} else {
			log::info!(
				"Withdrawal message submitted successfully to {}",
				dest_chain.state_machine_id().state_id
			);
			return Ok(());
		}
	}

	Err(anyhow::anyhow!("Failed to deliver post request"))
}
