//! Functions for updating configuration on pallets

use std::collections::BTreeMap;

use crate::{
	extrinsic::{send_extrinsic, send_unsigned_extrinsic, Extrinsic, InMemorySigner},
	runtime, SubstrateClient,
};
use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{
	events::{Event, StateMachineUpdated},
	host::StateMachine,
	messaging::CreateConsensusState,
};
use pallet_ismp_relayer::{
	message,
	withdrawal::{WithdrawalInputData, WithdrawalParams, WithdrawalProof},
};
use primitives::{HyperbridgeClaim, IsmpHost, IsmpProvider, WithdrawFundsResult};
use sp_core::{Pair, U256};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	tx::TxPayload,
	OnlineClient,
};

impl<T, C> SubstrateClient<T, C>
where
	T: IsmpHost + Send + Sync + Clone + 'static,
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId: From<sp_core::crypto::AccountId32>
		+ Into<C::Address>
		+ Encode
		+ Clone
		+ 'static
		+ Send
		+ Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
{
	pub async fn create_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), anyhow::Error> {
		let signer = InMemorySigner {
			account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
			signer: self.signer.clone(),
		};

		let call = message.encode();
		let call = Extrinsic::new("Ismp", "create_consensus_client", call)
			.encode_call_data(&self.client.metadata())?;
		let tx = Extrinsic::new("Sudo", "sudo", call);
		send_extrinsic(&self.client, signer, tx).await?;

		Ok(())
	}

	pub async fn set_host_manager_addresses(
		&self,
		addresses: BTreeMap<StateMachine, Vec<u8>>,
	) -> anyhow::Result<()> {
		let encoded_call =
			Extrinsic::new("StateMachineManager", "set_host_manger_addresses", addresses.encode())
				.encode_call_data(&self.client.metadata())?;
		let tx = Extrinsic::new("Sudo", "sudo", encoded_call);
		let signer = InMemorySigner::new(self.signer());
		send_extrinsic(&self.client, signer, tx).await?;

		Ok(())
	}
}

#[async_trait::async_trait]
impl<T, C> HyperbridgeClaim for SubstrateClient<T, C>
where
	T: IsmpHost + Send + Sync + Clone + 'static,
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId: From<sp_core::crypto::AccountId32>
		+ Into<C::Address>
		+ Encode
		+ Clone
		+ 'static
		+ Send
		+ Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
{
	async fn available_amount<P: IsmpProvider>(
		&self,
		client: &P,
		chain: &StateMachine,
	) -> anyhow::Result<U256> {
		Ok(relayer_account_balance(&self.client, chain.clone(), client.address()).await?)
	}

	/// Accumulate accrued fees on hyperbridge by submitting a claim proof
	async fn accumulate_fees(&self, proof: WithdrawalProof) -> anyhow::Result<()> {
		let tx = Extrinsic::new("Relayer", "accumulate_fees", proof.encode());
		send_unsigned_extrinsic(&self.client, tx).await?;

		Ok(())
	}

	/// Withdraw funds from hyperbridge and return the emitted post request
	async fn withdraw_funds<D: IsmpProvider>(
		&self,
		counterparty: &D,
		chain: StateMachine,
		gas_limit: u64,
	) -> anyhow::Result<WithdrawFundsResult> {
		let addr = runtime::api::storage()
			.relayer()
			.nonce(counterparty.address().as_slice(), &chain.into());
		let nonce =
			self.client.storage().at_latest().await?.fetch(&addr).await?.unwrap_or_default();

		let amount = relayer_account_balance(&self.client, chain, counterparty.address()).await?;
		let signature = {
			let message = message(nonce, chain, amount);
			counterparty.sign(&message)
		};

		let input_data = WithdrawalInputData { signature, dest_chain: chain, amount, gas_limit };

		let tx = Extrinsic::new("Relayer", "withdraw_fees", input_data.encode());
		let hash = send_unsigned_extrinsic(&self.client, tx)
			.await?
			.ok_or_else(|| anyhow!("Transaction submission failed"))?;
		let block_number = self
			.client
			.rpc()
			.header(Some(hash))
			.await?
			.ok_or_else(|| anyhow!("Header should exists"))?
			.number()
			.into();
		let mock_state_update = StateMachineUpdated {
			state_machine_id: counterparty.state_machine_id(),
			latest_height: block_number,
		};
		let event = self
			.query_ismp_events(block_number - 1, mock_state_update)
			.await?
			.into_iter()
			.find(|event| match event {
				Event::PostRequest(post) => {
					let condition =
						post.dest == chain && &post.from == &pallet_ismp_relayer::MODULE_ID;
					match post.dest {
						StateMachine::Kusama(_) |
						StateMachine::Polkadot(_) |
						StateMachine::Grandpa(_) |
						StateMachine::Beefy(_) => {
							if let Ok(decoded_data) = WithdrawalParams::decode(&mut &*post.data) {
								decoded_data.beneficiary_address == counterparty.address() &&
									condition
							} else {
								false
							}
						},
						StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
							let address = &post.data[1..33].to_vec();
							// abi encoding will pad address with 12 bytes
							address.ends_with(&counterparty.address()) && condition
						},
					}
				},
				_ => false,
			})
			.ok_or_else(|| anyhow!("Post Event should be present in block"))?;

		let Event::PostRequest(post) = event else { unreachable!() };

		Ok(WithdrawFundsResult { post, block: block_number })
	}
}

async fn relayer_account_balance<C: subxt::Config>(
	client: &OnlineClient<C>,
	chain: StateMachine,
	address: Vec<u8>,
) -> anyhow::Result<U256> {
	let addr = runtime::api::storage().relayer().fees(&chain.into(), address.as_slice());
	let balance = client
		.storage()
		.at_latest()
		.await?
		.fetch(&addr)
		.await?
		.map(|val| U256(val.0))
		.unwrap_or(U256::zero());

	Ok(balance)
}
