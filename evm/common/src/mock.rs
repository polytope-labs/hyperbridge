use ethabi::ethereum_types::H256;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
		StateMachineId,
	},
	error::Error,
	host::StateMachine,
	router::{IsmpRouter, PostResponse, Request, Response},
};
use std::time::Duration;

pub struct Host;

impl ismp::host::IsmpHost for Host {
	fn host_state_machine(&self) -> StateMachine {
		todo!()
	}

	fn latest_commitment_height(&self, _id: StateMachineId) -> Result<u64, Error> {
		todo!()
	}

	fn state_machine_commitment(
		&self,
		_height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		todo!()
	}

	fn consensus_update_time(
		&self,
		_consensus_state_id: ConsensusStateId,
	) -> Result<Duration, Error> {
		todo!()
	}

	fn state_machine_update_time(
		&self,
		_state_machine_height: StateMachineHeight,
	) -> Result<Duration, Error> {
		todo!()
	}

	fn consensus_client_id(
		&self,
		_consensus_state_id: ConsensusStateId,
	) -> Option<ConsensusClientId> {
		todo!()
	}

	fn consensus_state(&self, _consensus_state_id: ConsensusStateId) -> Result<Vec<u8>, Error> {
		todo!()
	}

	fn timestamp(&self) -> Duration {
		todo!()
	}

	fn is_state_machine_frozen(&self, _machine: StateMachineId) -> Result<(), Error> {
		todo!()
	}

	fn is_consensus_client_frozen(
		&self,
		_consensus_state_id: ConsensusStateId,
	) -> Result<(), Error> {
		todo!()
	}

	fn request_commitment(&self, _req: H256) -> Result<(), Error> {
		todo!()
	}

	fn response_commitment(&self, _req: H256) -> Result<(), Error> {
		todo!()
	}

	fn next_nonce(&self) -> u64 {
		todo!()
	}

	fn request_receipt(&self, _req: &Request) -> Option<()> {
		todo!()
	}

	fn response_receipt(&self, _res: &Response) -> Option<()> {
		todo!()
	}

	fn store_consensus_state_id(
		&self,
		_consensus_state_id: ConsensusStateId,
		_client_id: ConsensusClientId,
	) -> Result<(), Error> {
		todo!()
	}

	fn store_consensus_state(
		&self,
		_consensus_state_id: ConsensusStateId,
		_consensus_state: Vec<u8>,
	) -> Result<(), Error> {
		todo!()
	}

	fn store_unbonding_period(
		&self,
		_consensus_state_id: ConsensusStateId,
		_period: u64,
	) -> Result<(), Error> {
		todo!()
	}

	fn store_consensus_update_time(
		&self,
		_consensus_state_id: ConsensusStateId,
		_timestamp: Duration,
	) -> Result<(), Error> {
		todo!()
	}

	fn delete_request_receipt(&self, _req: &Request) -> Result<(), Error> {
		todo!()
	}

	fn delete_response_receipt(&self, _res: &PostResponse) -> Result<(), Error> {
		todo!()
	}

	fn store_state_machine_update_time(
		&self,
		_state_machine_height: StateMachineHeight,
		_timestamp: Duration,
	) -> Result<(), Error> {
		todo!()
	}

	fn store_state_machine_commitment(
		&self,
		_height: StateMachineHeight,
		_state: StateCommitment,
	) -> Result<(), Error> {
		todo!()
	}

	fn freeze_state_machine(&self, _height: StateMachineId) -> Result<(), Error> {
		todo!()
	}

	fn unfreeze_state_machine(&self, _state_machine: StateMachineId) -> Result<(), Error> {
		todo!()
	}

	fn freeze_consensus_client(&self, _consensus_state_id: ConsensusStateId) -> Result<(), Error> {
		todo!()
	}

	fn store_latest_commitment_height(&self, _height: StateMachineHeight) -> Result<(), Error> {
		todo!()
	}

	fn delete_request_commitment(&self, _req: &Request) -> Result<(), Error> {
		todo!()
	}

	fn delete_response_commitment(&self, _res: &PostResponse) -> Result<(), Error> {
		todo!()
	}

	fn store_request_receipt(&self, _req: &Request, _signer: &Vec<u8>) -> Result<(), Error> {
		todo!()
	}

	fn store_response_receipt(&self, _req: &Response, _signer: &Vec<u8>) -> Result<(), Error> {
		todo!()
	}

	fn consensus_client(&self, _id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
		todo!()
	}

	fn challenge_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
		todo!()
	}

	fn store_challenge_period(
		&self,
		_consensus_state_id: ConsensusStateId,
		_period: u64,
	) -> Result<(), Error> {
		todo!()
	}

	fn unbonding_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
		todo!()
	}

	fn ismp_router(&self) -> Box<dyn IsmpRouter> {
		todo!()
	}

	fn allowed_proxy(&self) -> Option<StateMachine> {
		todo!()
	}

	fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
		todo!()
	}
}

impl ismp::util::Keccak256 for Host {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		abi::{erc_20::Erc20, PingMessage, PingModule},
		SecretKey,
	};
	use anyhow::Context;
	use ethers::{
		prelude::{LocalWallet, MiddlewareBuilder, Signer},
		providers::{Http, Middleware, Provider, ProviderExt},
	};
	use futures::TryStreamExt;
	use hex_literal::hex;
	use ismp::host::{Ethereum, StateMachine};
	use ismp_solidity_abi::evm_host::EvmHost;
	use primitive_types::{H160, U256};
	use sp_core::Pair;
	use std::sync::Arc;

	#[tokio::test]
	#[ignore]
	async fn test_ping() -> anyhow::Result<()> {
		// dotenv::dotenv().ok();
		let op_url = std::env::var("OP_URL").unwrap_or(
			"https://opt-sepolia.g.alchemy.com/v2/qzZKMgRJ7zHxeUPoEvjYCmuAsJnx0oVP".into(),
		);
		let base_url = std::env::var("BASE_URL").unwrap_or(
			"https://base-sepolia.g.alchemy.com/v2/xLAACkUCNcEBquCQcsT7ypkaIfsTlQU3".into(),
		);
		let arb_url = std::env::var("ARB_URL").unwrap_or(
			"https://arb-sepolia.g.alchemy.com/v2/xd9UmE2ItdzJQMzivURMW5jyhlKLE8Qi".into(),
		);
		let geth_url = std::env::var("GETH_URL").unwrap_or(
			"https://eth-sepolia.g.alchemy.com/v2/tKtJs47xn9LPe8d99J0L06Ixg3bsHGIR".into(),
		);
		let bsc_url = std::env::var("BSC_URL").unwrap_or(
			"https://clean-capable-dew.bsc-testnet.quiknode.pro/bed456956996abb801b7ab44fdb3f6f63cd1a4ec/".into(),
		);

		let ping_addr = H160(hex!("d4812d6A3b9fB46feA314260Cbb61D57EBc71D7F"));

		let chains = vec![
			(StateMachine::Ethereum(Ethereum::ExecutionLayer), geth_url),
			(StateMachine::Ethereum(Ethereum::Arbitrum), arb_url),
			(StateMachine::Ethereum(Ethereum::Optimism), op_url),
			(StateMachine::Ethereum(Ethereum::Base), base_url),
			(StateMachine::Bsc, bsc_url),
		];

		let stream = futures::stream::iter(chains.clone().into_iter().map(Ok::<_, anyhow::Error>));

		stream
			.try_for_each_concurrent(None, |(chain, url)| {
				let chains_clone = chains.clone();
				async move {
					let signer = sp_core::ecdsa::Pair::from_seed_slice(&hex!(
						"6456101e79abe59d2308d63314503446857d4f1f949468bf5627e86e3d6adebd"
					))?;
					let provider = Arc::new(Provider::<Http>::try_connect(&url).await?);
					let signer =
						LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
							.with_chain_id(provider.get_chainid().await?.low_u64());
					let client = Arc::new(provider.with_signer(signer));
					let ping = PingModule::new(ping_addr.clone(), client.clone());

					let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
					dbg!((&chain, &host_addr));

					let host = EvmHost::new(host_addr, client.clone());
					let erc_20 = Erc20::new(
						host.dai().await.context(format!("Error in {chain:?}"))?,
						client.clone(),
					);
					let call = erc_20.approve(host_addr, U256::max_value());
					let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
					call.gas(gas)
						.send()
						.await
						.context(format!("Failed to send approval for {host_addr} in {chain:?}"))?
						.await
						.context(format!("Failed to approve {host_addr} in {chain:?}"))?;

					for (chain, _) in chains_clone.iter().filter(|(c, _)| chain != *c) {
						for _ in 0..10 {
							let call = ping.ping(PingMessage {
								dest: chain.to_string().as_bytes().to_vec().into(),
								module: ping_addr.clone().into(),
								timeout: 10 * 60 * 60,
								fee: U256::from(30_000_000_000_000_000_000u128),
								count: U256::from(100),
							});
							let gas = call
								.estimate_gas()
								.await
								.context(format!("Failed to estimate gas in {chain:?}"))?;
							let call = call.gas(gas);
							let Ok(tx) = call.send().await else { continue };
							let receipt = tx
								.await
								.context(format!("Failed to execute ping message on {chain:?}"))?;

							assert!(receipt.is_some());
						}
					}

					Ok(())
				}
			})
			.await?;

		Ok(())
	}
}
