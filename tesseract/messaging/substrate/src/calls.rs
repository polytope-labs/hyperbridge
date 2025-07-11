//! Functions for updating configuration on pallets

use crate::{
	extrinsic::{send_unsigned_extrinsic, system_dry_run_unsigned, Extrinsic, InMemorySigner},
	SubstrateClient,
};
use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{
	events::{Event, StateMachineUpdated},
	host::StateMachine,
	messaging::CreateConsensusState,
};
use pallet_hyperbridge::WithdrawalRequest;
use pallet_ismp::{child_trie::CHILD_TRIE_PREFIX, offchain::LeafIndexAndPos};
use pallet_ismp_host_executive::HostParam;
use pallet_ismp_relayer::{
	message,
	withdrawal::{Key, WithdrawalInputData, WithdrawalProof},
};
use pallet_state_coprocessor::impls::GetRequestsWithProof;
use sp_core::{
	storage::{ChildInfo, StorageData, StorageKey},
	H256, U256,
};
use std::{collections::BTreeMap, sync::Arc};
use subxt::{
	config::{
		ExtrinsicParams, Header,
	},
	ext::{
		subxt_rpcs::{rpc_params, methods::legacy::DryRunResult}
	},
	tx::Payload,
	utils::AccountId32,
	OnlineClient,
};
use polkadot_sdk::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner};
use polkadot_sdk::sp_core::{crypto, Pair};
use subxt_utils::{relayer_account_balance_storage_key, relayer_nonce_storage_key, send_extrinsic};
use tesseract_primitives::{
	HandleGetResponse, HyperbridgeClaim, IsmpProvider, WithdrawFundsResult,
};

#[derive(codec::Encode, codec::Decode)]
pub struct RequestMetadata {
	/// Information about where it's stored in the offchain db
	pub mmr: LeafIndexAndPos,
	/// Other metadata about the request
	pub meta: ismp::dispatcher::FeeMetadata<AccountId32, u128>,
	/// Relayer Fee claimed?
	pub claimed: bool,
}

impl<C> SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId:
		From<crypto::AccountId32> + Into<C::Address> + Encode + Clone + 'static + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<<C as subxt::Config>::Hash>,
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
		send_extrinsic(&self.client, signer, tx, None).await?;

		Ok(())
	}

	pub async fn set_host_params(
		&self,
		params: BTreeMap<StateMachine, HostParam<u128>>,
	) -> anyhow::Result<()> {
		let encoded_call = Extrinsic::new("HostExecutive", "set_host_params", params.encode())
			.encode_call_data(&self.client.metadata())?;
		let tx = Extrinsic::new("Sudo", "sudo", encoded_call);
		let signer = InMemorySigner::new(self.signer());
		send_extrinsic(&self.client, signer, tx, None).await?;

		Ok(())
	}
}

#[async_trait::async_trait]
impl<C> HyperbridgeClaim for SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId:
		From<crypto::AccountId32> + Into<C::Address> + Encode + Clone + 'static + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<<C as subxt::Config>::Hash>,
{
	async fn available_amount(
		&self,
		client: Arc<dyn IsmpProvider>,
		chain: &StateMachine,
	) -> anyhow::Result<U256> {
		Ok(relayer_account_balance(&self.client, chain.clone(), client.address()).await?)
	}

	/// Accumulate accrued fees on hyperbridge by submitting a claim proof
	async fn accumulate_fees(&self, proof: WithdrawalProof) -> anyhow::Result<()> {
		let extrinsic = Extrinsic::new("Relayer", "accumulate_fees", proof.encode());
		let encoded_call = extrinsic.encode_call_data(&self.client.metadata())?;
		let uncompressed_len = encoded_call.len();
		let max_compressed_size = zstd_safe::compress_bound(uncompressed_len);
		let mut buffer = vec![0u8; max_compressed_size];
		let compressed_call_len = zstd_safe::compress(&mut buffer[..], &encoded_call, 3)
			.map_err(|_| anyhow!("Call compression failed"))?;
		// If compression saving is less than 15% submit the uncompressed call
		if (uncompressed_len.saturating_sub(compressed_call_len) * 100 / uncompressed_len) < 20usize
		{
			send_unsigned_extrinsic(&self.client, extrinsic, true).await?;
		} else {
			let compressed_call = buffer[0..compressed_call_len].to_vec();
			let call = (compressed_call, uncompressed_len as u32).encode();
			let extrinsic = Extrinsic::new("CallDecompressor", "decompress_call", call);
			send_unsigned_extrinsic(&self.client, extrinsic, true).await?;
		}

		Ok(())
	}

	/// Withdraw funds from hyperbridge and return the emitted post request
	async fn withdraw_funds(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
		chain: StateMachine,
	) -> anyhow::Result<WithdrawFundsResult> {
		let key = relayer_nonce_storage_key(counterparty.address(), chain);
		let raw_value = self.client.storage().at_latest().await?.fetch_raw(&key).await?;
		let nonce =
			if let Some(raw_value) = raw_value { Decode::decode(&mut &*raw_value)? } else { 0u64 };

		let signature = {
			let message = message(nonce, chain);
			counterparty.sign(&message)
		};

		let input_data = WithdrawalInputData { signature, dest_chain: chain };

		let tx = Extrinsic::new("Relayer", "withdraw_fees", input_data.encode());
		// Wait for finalization so we still get the correct block with the post request event even
		// if a reorg happens
		let (hash, _) = send_unsigned_extrinsic(&self.client, tx, true)
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
						s if s.is_substrate() => {
							if let Ok(pallet_hyperbridge::Message::WithdrawRelayerFees(
								WithdrawalRequest { account, .. },
							)) = pallet_hyperbridge::Message::<AccountId32, u128>::decode(
								&mut &*post.body,
							) {
								account.0.to_vec() == counterparty.address() && condition
							} else {
								false
							}
						},
						s if s.is_evm() => {
							let address = &post.body[1..33].to_vec();
							// abi encoding will pad address with 12 bytes
							address.ends_with(&counterparty.address()) && condition
						},
						_ => false,
					}
				},
				_ => false,
			})
			.ok_or_else(|| anyhow!("Post Event should be present in block"))?;

		let Event::PostRequest(post) = event else { unreachable!() };

		Ok(WithdrawFundsResult { post, block: block_number })
	}

	async fn check_claimed(&self, key: Key) -> anyhow::Result<bool> {
		let params = match key {
			Key::Request(req) => {
				let key = self.req_commitments_key(req);
				let child_storage_key =
					ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
				let storage_key = StorageKey(key);

				rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None]
			},
			Key::Response { response_commitment, .. } => {
				let key = self.res_commitments_key(response_commitment);
				let child_storage_key =
					ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
				let storage_key = StorageKey(key);
				rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None]
			},
		};

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		let data = response.ok_or_else(|| anyhow!("Request fee metadata query returned None"))?;
		let leaf_meta = RequestMetadata::decode(&mut &*data.0)?;

		Ok(leaf_meta.claimed)
	}
}

#[async_trait::async_trait]
impl<C> HandleGetResponse for SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId:
		From<crypto::AccountId32> + Into<C::Address> + Encode + Clone + 'static + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
{
	async fn submit_get_response(&self, msg: GetRequestsWithProof) -> anyhow::Result<()> {
		let tx = Extrinsic::new("StateCoprocessor", "handle_unsigned", msg.encode());
		let _ = send_unsigned_extrinsic(&self.client, tx, false)
			.await?
			.ok_or_else(|| anyhow!("Transaction submission failed"))?;
		Ok(())
	}

	async fn dry_run_submission(&self, msg: GetRequestsWithProof) -> anyhow::Result<()> {
		let tx = Extrinsic::new("StateCoprocessor", "handle_unsigned", msg.encode());
		match system_dry_run_unsigned(&self.client, tx).await? {
			DryRunResult::Success => Ok(()),
			_ => Err(anyhow!("Tracing of get response message returned an error")),
		}
	}
}

async fn relayer_account_balance<C: subxt::Config>(
	client: &OnlineClient<C>,
	chain: StateMachine,
	address: Vec<u8>,
) -> anyhow::Result<U256> {
	let key = relayer_account_balance_storage_key(chain, address);

	let raw_value = client.storage().at_latest().await?.fetch_raw(&key).await?;
	let balance = if let Some(raw_value) = raw_value {
		Decode::decode(&mut &*raw_value)?
	} else {
		Default::default()
	};

	Ok(balance)
}
