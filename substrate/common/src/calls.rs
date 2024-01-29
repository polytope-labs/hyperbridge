//! Functions for updating configuration on pallets

use crate::{
	extrinsic::{send_extrinsic, send_unsigned_extrinsic, Extrinsic, InMemorySigner},
	SubstrateClient,
};
use anyhow::anyhow;
use codec::{Decode, Encode};
use hex_literal::hex;
use ismp::{
	events::{Event, StateMachineUpdated},
	host::StateMachine,
	messaging::CreateConsensusState,
};
use pallet_relayer_fees::{
	message,
	withdrawal::{WithdrawalInputData, WithdrawalParams, WithdrawalProof},
};
use primitives::{HyperbridgeClaim, IsmpHost, IsmpProvider, WithdrawFundsResult};
use sp_core::{twox_64, Pair};
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
	T: IsmpHost + Send + Sync + Clone,
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
		let nonce = self.get_nonce().await?;
		send_extrinsic(&self.client, signer, tx, nonce).await?;

		Ok(())
	}
}

#[async_trait::async_trait]
impl<T, C> HyperbridgeClaim for SubstrateClient<T, C>
where
	T: IsmpHost + Send + Sync + Clone,
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
	/// Accumulate accrued fees on hyperbridge by submitting a claim proof
	async fn accumulate_fees(&self, proof: WithdrawalProof) -> anyhow::Result<()> {
		let tx = Extrinsic::new("RelayerFees", "accumulate_fees", proof.encode());
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
		let mut nonce_key =
			hex!("0f58da11a3e360dcc90475225459bcf9718368a0ace36e2b1b8b6dbd7f8093c0").to_vec();
		let hashed_key = twox_64(&counterparty.address());
		nonce_key.extend_from_slice(&hashed_key);
		let response = self
			.client
			.rpc()
			.storage(&nonce_key, None)
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch Nonce"))?;
		let nonce: u64 = codec::Decode::decode(&mut response.0.as_slice())?;
		let amount = relayer_account_balance(&self.client, chain, counterparty.address()).await?;
		let signature = {
			let message = message(nonce, chain, amount);
			counterparty.sign(&message)
		};

		let input_data = WithdrawalInputData { signature, dest_chain: chain, amount, gas_limit };

		let tx = Extrinsic::new("RelayerFees", "withdraw_fees", input_data.encode());
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
			latest_height: block_number + 1,
		};
		let event = self
			.query_ismp_events(block_number - 1, mock_state_update)
			.await?
			.into_iter()
			.find(|event| match event {
				Event::PostRequest(post) => {
					let condition =
						post.dest == chain && &post.from == &pallet_relayer_fees::MODULE_ID;
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
						StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc =>
							&post.data[..20] == &counterparty.address() && condition,
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
) -> anyhow::Result<u128> {
	let mut relayer_fees =
		hex!("0f58da11a3e360dcc90475225459bcf90f58da11a3e360dcc90475225459bcf9").to_vec();
	let encoded_state_machine = twox_64(&chain.encode());
	let encoded_relayer_address = twox_64(&address.encode());
	relayer_fees.extend_from_slice(&encoded_state_machine);
	relayer_fees.extend_from_slice(&encoded_relayer_address);
	let response = client
		.rpc()
		.storage(&relayer_fees, None)
		.await?
		.ok_or_else(|| anyhow!("Failed to fetch Relayer Balance"))?;
	let balance: u128 = codec::Decode::decode(&mut response.0.as_slice())?;
	Ok(balance)
}
