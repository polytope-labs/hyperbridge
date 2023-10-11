use ethabi::ethereum_types::H256;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
		StateMachineId,
	},
	error::Error,
	host::StateMachine,
	router::{IsmpRouter, Request},
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

	fn is_state_machine_frozen(&self, _machine: StateMachineHeight) -> Result<(), Error> {
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

	fn next_nonce(&self) -> u64 {
		todo!()
	}

	fn request_receipt(&self, _req: &Request) -> Option<()> {
		todo!()
	}

	fn response_receipt(&self, _res: &Request) -> Option<()> {
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

	fn freeze_state_machine(&self, _height: StateMachineHeight) -> Result<(), Error> {
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

	fn store_request_receipt(&self, _req: &Request) -> Result<(), Error> {
		todo!()
	}

	fn store_response_receipt(&self, _req: &Request) -> Result<(), Error> {
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

	fn allowed_proxies(&self) -> Vec<StateMachine> {
		todo!()
	}

	fn store_allowed_proxies(&self, _allowed: Vec<StateMachine>) {
		todo!()
	}

	fn unbonding_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
		todo!()
	}

	fn ismp_router(&self) -> Box<dyn IsmpRouter> {
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
		abi::{PingMessage, PingModule},
		SecretKey,
	};
	use ethers::{
		prelude::{LocalWallet, MiddlewareBuilder, Signer},
		providers::{Provider, Ws},
	};
	use hex_literal::hex;
	use ismp::host::{Ethereum, StateMachine};
	use primitive_types::H160;
	use sp_core::Pair;
	use std::sync::Arc;

	#[tokio::test]
	#[ignore]
	async fn test_ping() -> anyhow::Result<()> {
		dotenv::dotenv().ok();
		let op_url = std::env::var("OP_URL").expect("OP_URL must be set.");
		let base_url = std::env::var("BASE_URL").expect("OP_URL must be set.");
		let arb_url = std::env::var("ARB_URL").expect("OP_URL must be set.");
		let geth_url = std::env::var("GETH_URL").expect("OP_URL must be set.");
		let chains = vec![
			(
				StateMachine::Ethereum(Ethereum::ExecutionLayer),
				H160(hex!("53920d815e1518eebDa3c09D614A6ce59d9fb4B0")),
				geth_url,
				5u64,
			),
			(
				StateMachine::Ethereum(Ethereum::Arbitrum),
				H160(hex!("4E97A39f8Be6b568Df76dc7e9B141e53c1e519EF")),
				arb_url,
				421613,
			),
			(
				StateMachine::Ethereum(Ethereum::Optimism),
				H160(hex!("617Ba1259FDFAc28c2B192B50057f3D62FeCB33b")),
				op_url,
				420,
			),
			(
				StateMachine::Ethereum(Ethereum::Base),
				H160(hex!("F1a722eC517e5F4dCb78ef09908efb52dB6D6180")),
				base_url,
				84531,
			),
		];

		let signer = sp_core::ecdsa::Pair::from_seed_slice(&hex!(
			"2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622"
		))?;

		for (chain, address, url, chain_id) in chains.iter() {
			let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
				.with_chain_id(*chain_id);
			let provider = Arc::new(Provider::<Ws>::connect(url).await?);
			let client = Arc::new(provider.with_signer(signer));

			let ping = PingModule::new(address.clone(), client);

			for (chain, address, _, _) in chains.iter().filter(|(c, _, _, _)| *chain != *c) {
				let receipt = ping
					.ping(PingMessage {
						dest: chain.to_string().as_bytes().to_vec().into(),
						module: address.clone().into(),
					})
					.gas(10_000_000)
					.send()
					.await?
					.await?;

				dbg!(receipt);
			}
		}

		Ok(())
	}
}
