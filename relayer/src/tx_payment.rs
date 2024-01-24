use crate::{
	config::{AnyClient, HyperbridgeConfig},
	create_client_map,
};
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
	wait_for_challenge_period, HyperbridgeClaim, IsmpProvider, Query, Reconnect,
	WithdrawFundsResult,
};
use std::{collections::HashMap, str::FromStr};
use tesseract_bnb_pos::KeccakHasher;
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
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,
	/// Source chain
	pub source: String,
	/// Destination chain
	pub dest: String,
	/// Accumulate fees for these number of days from the current date
	pub days: Option<u64>,
	/// Source chain height from which to fetch proofs
	pub source_height: u64,
	/// Destination chain height from which to fetch proofs
	pub dest_height: u64,
	/// Gas limit for executing withdrawal requests on both chains
	pub gas_limit: Option<u64>,
}

impl AccumulateFees {
	/// Accumulate fees accrued through deliveries from source to dest and dest to source
	pub async fn accumulate_fees(&self) -> anyhow::Result<()> {
		let config = {
			let toml = tokio::fs::read_to_string(&self.config).await?;
			toml::from_str::<HyperbridgeConfig>(&toml)?
		};

		let HyperbridgeConfig { hyperbridge: hyperbridge_config, .. } = config.clone();

		let mut hyperbridge = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		let hyperbridge_nonce_provider = hyperbridge.initialize_nonce().await?;
		hyperbridge.set_nonce_provider(hyperbridge_nonce_provider.clone());
		let (clients, _) = create_client_map(config).await?;

		let tx_payment = TransactionPayment::initialize().await?;

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
		// Create claim proof for deliveries from source to dest
		let claim_proof = tx_payment
			.create_claim_proof(self.source_height, self.dest_height, source, dest)
			.await?;
		hyperbridge.accumulate_fees(claim_proof.clone()).await?;
		tx_payment.delete_claimed_entries(claim_proof).await?;
		// Create claim proof for deliveries from dest to source
		let claim_proof = tx_payment
			.create_claim_proof(self.dest_height, self.source_height, dest, source)
			.await?;
		hyperbridge.accumulate_fees(claim_proof.clone()).await?;
		tx_payment.delete_claimed_entries(claim_proof).await?;

		self.withdraw(&hyperbridge, clients).await?;
		Ok(())
	}

	pub async fn withdraw<C: IsmpProvider + HyperbridgeClaim>(
		&self,
		hyperbridge: &C,
		mut clients: HashMap<StateMachine, AnyClient>,
	) -> anyhow::Result<()> {
		let source_chain =
			StateMachine::from_str(&self.source).expect("Invalid Source State Machine provided");
		let dest_chain =
			StateMachine::from_str(&self.dest).expect("Invalid Dest State Machine provided");

		let chain_a = clients
			.get_mut(&source_chain)
			.ok_or_else(|| anyhow!("Client not found for state machine"))?;
		let result = hyperbridge
			.withdraw_funds(chain_a, source_chain, self.gas_limit.unwrap_or_default())
			.await?;

		// Wait for state machine update
		deliver_post_request(chain_a, hyperbridge, result).await?;

		let chain_b = clients
			.get_mut(&dest_chain)
			.ok_or_else(|| anyhow!("Client not found for state machine"))?;
		let result = hyperbridge
			.withdraw_funds(chain_b, dest_chain, self.gas_limit.unwrap_or_default())
			.await?;

		// Wait for state machine update
		deliver_post_request(chain_b, hyperbridge, result).await
	}
}

async fn deliver_post_request<C: IsmpProvider + Reconnect, D: IsmpProvider>(
	dest_chain: &mut C,
	hyperbridge: &D,
	result: WithdrawFundsResult,
) -> anyhow::Result<()> {
	let mut stream = dest_chain
		.state_machine_update_notification(hyperbridge.state_machine_id())
		.await?;
	let mut delivered = false;
	loop {
		while let Some(Ok(event)) = stream.next().await {
			if event.latest_height >= result.block {
				let challenge_period = dest_chain
					.query_challenge_period(event.state_machine_id.consensus_state_id)
					.await?;
				let last_consensus_update = dest_chain
					.query_consensus_update_time(event.state_machine_id.consensus_state_id)
					.await?;
				println!("Waiting for challenge period to elapse");
				wait_for_challenge_period(dest_chain, last_consensus_update, challenge_period)
					.await?;
				let query = Query {
					source_chain: result.post.source,
					dest_chain: result.post.dest,
					nonce: result.post.nonce,
					commitment: hash_request::<KeccakHasher>(&Request::Post(result.post.clone())),
				};
				let proof =
					hyperbridge.query_requests_proof(event.latest_height, vec![query]).await?;
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

				dest_chain.submit(vec![Message::Request(msg)]).await?;
				delivered = true;
				break
			}
		}
		if !delivered {
			println!("Trying to resubmit withdrawal to destination");
			dest_chain.reconnect().await?;
			stream = dest_chain
				.state_machine_update_notification(hyperbridge.state_machine_id())
				.await?;
		} else {
			break
		}
	}
	Ok(())
}
