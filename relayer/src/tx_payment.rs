use crate::{config::HyperbridgeConfig, create_client_map, logging};
use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage},
	router::Request,
	util::hash_request,
};
use primitives::{
	wait_for_challenge_period, HyperbridgeClaim, IsmpProvider, Query, WithdrawFundsResult,
};
use std::{collections::HashMap, str::FromStr, time::Duration};
use tesseract_any_client::AnyClient;
use tesseract_bsc_pos::KeccakHasher;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
use transaction_payment::TransactionPayment;

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
	/// Source chain
	#[arg(short, long)]
	pub source: String,
	/// Destination chain
	#[arg(short, long)]
	pub dest: String,
	/// Accumulate fees and withdraw the funds
	#[arg(short, long)]
	pub withdraw: bool,
	/// Gas limit for executing withdrawal requests on both chains
	#[arg(short, long)]
	pub gas_limit: Option<u64>,
}

impl AccumulateFees {
	/// Accumulate fees accrued through deliveries from source to dest and dest to source
	pub async fn accumulate_fees(&self, config_path: String) -> anyhow::Result<()> {
		logging::setup()?;
		let config = HyperbridgeConfig::parse_conf(&config_path).await?;

		let HyperbridgeConfig { hyperbridge: hyperbridge_config, .. } = config.clone();

		let hyperbridge: tesseract_substrate::SubstrateClient<
			tesseract_beefy::BeefyHost<Blake2SubstrateChain, KeccakSubstrateChain>,
			KeccakSubstrateChain,
		> = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		let clients = create_client_map(config).await?;

		let tx_payment = TransactionPayment::initialize().await?;
		log::info!("Initialized database");
		let source_chain =
			StateMachine::from_str(&self.source).expect("Invalid Source State Machine provided");
		let dest_chain =
			StateMachine::from_str(&self.dest).expect("Invalid Dest State Machine provided");
		let source = clients
			.get(&source_chain)
			.ok_or_else(|| anyhow!("Client not found for source state machine"))?;
		let dest = clients
			.get(&dest_chain)
			.ok_or_else(|| anyhow!("Client not found for dest state machine"))?;
		let source_height = hyperbridge.query_latest_height(source.state_machine_id()).await?;
		let dest_height = hyperbridge.query_latest_height(dest.state_machine_id()).await?;
		// Create claim proof for deliveries from source to dest
		log::info!("Creating withdrawal proof from db for deliveries from {source_chain:?}->{dest_chain:?}");
		let claim_proof = tx_payment
			.create_claim_proof(source_height.into(), dest_height.into(), source, dest)
			.await?;
		let withdraw_from_a = if !claim_proof.commitments.is_empty() {
			log::info!("Submitting proof for {source_chain:?}->{dest_chain:?} to hyperbridge");
			hyperbridge.accumulate_fees(claim_proof.clone()).await?;
			log::info!("Proof sucessfully submitted");
			tx_payment.delete_claimed_entries(claim_proof).await?;
			true
		} else {
			log::info!("No deliveries found in db");
			false
		};
		// Create claim proof for deliveries from dest to source
		log::info!("Creating withdrawal proof from db for deliveries from {dest_chain:?}->{source_chain:?}");
		let claim_proof = tx_payment
			.create_claim_proof(dest_height.into(), source_height.into(), dest, source)
			.await?;
		let withdraw_from_b = if !claim_proof.commitments.is_empty() {
			log::info!("Submitting proof for {dest_chain:?}->{source_chain:?} to hyperbridge");
			hyperbridge.accumulate_fees(claim_proof.clone()).await?;
			log::info!("Proof sucessfully submitted");
			tx_payment.delete_claimed_entries(claim_proof).await?;
			true
		} else {
			log::info!("No deliveries found in db");
			false
		};
		if self.withdraw {
			self.withdraw(&hyperbridge, clients, withdraw_from_a, withdraw_from_b).await?;
		}
		Ok(())
	}

	pub async fn withdraw<C: IsmpProvider + HyperbridgeClaim>(
		&self,
		hyperbridge: &C,
		mut clients: HashMap<StateMachine, AnyClient>,
		withdraw_from_a: bool,
		withdraw_from_b: bool,
	) -> anyhow::Result<()> {
		let source_chain =
			StateMachine::from_str(&self.source).expect("Invalid Source State Machine provided");
		let dest_chain =
			StateMachine::from_str(&self.dest).expect("Invalid Dest State Machine provided");
		if withdraw_from_a {
			let chain_a = clients
				.get_mut(&source_chain)
				.ok_or_else(|| anyhow!("Client not found for state machine"))?;
			log::info!("Submitting withdrawal request to hyperbridge");
			let result = hyperbridge
				.withdraw_funds(chain_a, source_chain, self.gas_limit.unwrap_or_default())
				.await?;
			log::info!("Request submitted to hyperbridge successfully");
			log::info!("Starting delivery of withdrawal message to {}", source_chain);
			// Wait for state machine update
			deliver_post_request(chain_a, hyperbridge, result).await?;
		}

		if withdraw_from_b {
			let chain_b = clients
				.get_mut(&dest_chain)
				.ok_or_else(|| anyhow!("Client not found for state machine"))?;
			let result = hyperbridge
				.withdraw_funds(chain_b, dest_chain, self.gas_limit.unwrap_or_default())
				.await?;
			log::info!("Starting delivery of withdrawal message to {}", dest_chain);
			// Wait for state machine update
			deliver_post_request(chain_b, hyperbridge, result).await?;
		}

		Ok(())
	}
}

async fn deliver_post_request<C: IsmpProvider, D: IsmpProvider>(
	dest_chain: &mut C,
	hyperbridge: &D,
	result: WithdrawFundsResult,
) -> anyhow::Result<()> {
	let mut stream = dest_chain
		.state_machine_update_notification(hyperbridge.state_machine_id())
		.await?;
	let mut delivered = false;
	let mut retries = 0;
	loop {
		while let Some(Ok(event)) = stream.next().await {
			log::info!("Waiting for state machine update");
			if event.latest_height >= result.block {
				log::info!("Found a valid state machine update");
				let challenge_period = dest_chain
					.query_challenge_period(event.state_machine_id.consensus_state_id)
					.await?;
				let height =
					StateMachineHeight { id: event.state_machine_id, height: event.latest_height };
				let last_consensus_update =
					dest_chain.query_state_machine_update_time(height).await?;
				log::info!("Waiting for challenge period to elapse");
				wait_for_challenge_period(dest_chain, last_consensus_update, challenge_period)
					.await?;
				let query = Query {
					source_chain: result.post.source,
					dest_chain: result.post.dest,
					nonce: result.post.nonce,
					commitment: hash_request::<KeccakHasher>(&Request::Post(result.post.clone())),
				};
				log::info!("Queryng request proof from hyperbridge");
				let proof =
					hyperbridge.query_requests_proof(event.latest_height, vec![query]).await?;
				log::info!("Successfully queried request proof from hyperbridge");
				let msg = RequestMessage {
					requests: vec![result.post.clone()],
					proof: Proof {
						height: StateMachineHeight {
							id: hyperbridge.state_machine_id(),
							height: event.latest_height,
						},
						proof,
					},
					signer: dest_chain.address(),
				};

				log::info!(
					"Submitting post request to message to {:?}",
					dest_chain.state_machine_id().state_id
				);

				if let Err(e) = dest_chain.submit(vec![Message::Request(msg)]).await {
					log::info!(
						"Encountered error trying to submit withdrawal request to {:?}\n {e:?}",
						dest_chain.state_machine_id().state_id
					);
				} else {
					log::info!(
						"Withdrawal message submitted successfully to {:?}",
						dest_chain.state_machine_id().state_id
					);
					delivered = true;
					break
				}
			}
		}
		if !delivered && retries < 5 {
			// Try again
			tokio::time::sleep(Duration::from_secs(30)).await;
			retries += 1;
		} else {
			break
		}
	}
	Ok(())
}
