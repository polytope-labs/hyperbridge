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
	wait_for_challenge_period, Cost, HyperbridgeClaim, IsmpProvider, Query, WithdrawFundsResult,
};
use sp_core::U256;
use std::{collections::HashMap, str::FromStr};
use tesseract_bsc_pos::KeccakHasher;
use tesseract_client::AnyClient;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
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
}

impl AccumulateFees {
	/// Accumulate fees accrued through deliveries from source to dest and dest to source
	pub async fn accumulate_fees(&self, config_path: String, db: String) -> anyhow::Result<()> {
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

		// early return if withdrawing
		if self.withdraw {
			self.withdraw(&hyperbridge, clients).await?;
			return Ok(())
		}

		let tx_payment = TransactionPayment::initialize(&db).await?;
		log::info!("Initialized database");

		for delivery in tx_payment.distinct_deliveries().await? {
			let source_chain = StateMachine::from_str(&delivery.source_chain)
				.expect("Invalid Source State Machine provided");
			let dest_chain = StateMachine::from_str(&delivery.dest_chain)
				.expect("Invalid Dest State Machine provided");

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
			if !claim_proof.commitments.is_empty() {
				log::info!("Submitting proof for {source_chain:?}->{dest_chain:?} to hyperbridge");
				hyperbridge.accumulate_fees(claim_proof.clone()).await?;
				log::info!("Proof sucessfully submitted");
				tx_payment.delete_claimed_entries(claim_proof).await?;
			} else {
				log::info!("No deliveries found in db for {source_chain:?}->{dest_chain:?}");
			};
			// Create claim proof for deliveries from dest to source
			log::info!("Creating withdrawal proof from db for deliveries from {dest_chain:?}->{source_chain:?}");
			let claim_proof = tx_payment
				.create_claim_proof(dest_height.into(), source_height.into(), dest, source)
				.await?;
			if !claim_proof.commitments.is_empty() {
				log::info!("Submitting proof for {dest_chain:?}->{source_chain:?} to hyperbridge");
				hyperbridge.accumulate_fees(claim_proof.clone()).await?;
				log::info!("Proof sucessfully submitted");
				tx_payment.delete_claimed_entries(claim_proof).await?;
			} else {
				log::info!("No deliveries found in db for {dest_chain:?}->{source_chain:?}");
			};
		}

		Ok(())
	}

	pub async fn withdraw<C: IsmpProvider + HyperbridgeClaim>(
		&self,
		hyperbridge: &C,
		clients: HashMap<StateMachine, AnyClient>,
	) -> anyhow::Result<()> {
		for chain in clients.keys().cloned() {
			let client = clients
				.get(&chain)
				.ok_or_else(|| anyhow!("Client not found for state machine"))?;
			let amount = hyperbridge.available_amount(client, &chain).await?;

			if amount == U256::zero() {
				continue;
			}

			log::info!("Submitting withdrawal request to {chain:?} for amount ${}", Cost(amount));
			let result = hyperbridge
				.withdraw_funds(client, chain, self.gas_limit.unwrap_or_default())
				.await?;
			log::info!("Request submitted to hyperbridge successfully");
			log::info!("Starting delivery of withdrawal message to {:?}", chain);
			// Wait for state machine update
			deliver_post_request(client, hyperbridge, result).await?;
		}

		Ok(())
	}
}

async fn deliver_post_request<C: IsmpProvider, D: IsmpProvider>(
	dest_chain: &C,
	hyperbridge: &D,
	result: WithdrawFundsResult,
) -> anyhow::Result<()> {
	let mut stream = dest_chain
		.state_machine_update_notification(hyperbridge.state_machine_id())
		.await?;

	while let Some(Ok(event)) = stream.next().await {
		log::info!("Waiting for state machine update");
		if event.latest_height < result.block {
			continue
		}
		log::info!("Found a valid state machine update");
		let challenge_period = dest_chain
			.query_challenge_period(event.state_machine_id.consensus_state_id)
			.await?;
		let height = StateMachineHeight { id: event.state_machine_id, height: event.latest_height };
		let last_consensus_update = dest_chain.query_state_machine_update_time(height).await?;
		log::info!("Waiting for challenge period to elapse");
		wait_for_challenge_period(dest_chain, last_consensus_update, challenge_period).await?;
		let query = Query {
			source_chain: result.post.source,
			dest_chain: result.post.dest,
			nonce: result.post.nonce,
			commitment: hash_request::<KeccakHasher>(&Request::Post(result.post.clone())),
		};
		log::info!("Querying request proof from hyperbridge at {}", event.latest_height);
		let proof = hyperbridge.query_requests_proof(event.latest_height, vec![query]).await?;
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

		log::info!("Submitting post request to {:?}", dest_chain.state_machine_id().state_id);

		let mut count = 5;
		while count != 0 {
			if let Err(e) = dest_chain.submit(vec![Message::Request(msg.clone())]).await {
				log::info!(
					"Encountered error trying to submit withdrawal request to {:?}.\n{e:?}\nWill retry {count} more times.",
					dest_chain.state_machine_id().state_id
				);
				count -= 1;
			} else {
				log::info!(
					"Withdrawal message submitted successfully to {:?}",
					dest_chain.state_machine_id().state_id
				);
				return Ok(())
			}
		}

		break
	}
	Ok(())
}
