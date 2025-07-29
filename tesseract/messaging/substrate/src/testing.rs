// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Testing utilities

use codec::Encode;
use futures::stream::StreamExt;
use sp_core::H256;
use subxt::{
	config::{ExtrinsicParams, HashFor, Hasher, Header},
	dynamic::Value,
	events::EventDetails,
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};

use pallet_ismp_demo::{EvmParams, GetRequest, TransferParams};
use subxt_utils::{
	send_extrinsic,
	values::{
		account_vec_to_value, evm_params_to_value, get_request_ismp_demo_to_value,
		transfer_params_to_value,
	},
};

use crate::{extrinsic::InMemorySigner, SubstrateClient};

impl<C> SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
	C::AccountId: From<AccountId32> + Into<C::Address> + Encode + Clone + 'static + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<HashFor<C>>,
{
	pub fn latest_height(&self) -> u64 {
		self.initial_height
	}

	pub async fn transfer(
		&self,
		params: TransferParams<C::AccountId, u128>,
	) -> Result<HashFor<C>, anyhow::Error> {
		let call = subxt::dynamic::tx(
			"IsmpDemo",
			"transfer",
			vec![transfer_params_to_value::<C>(&params)],
		);

		let signer = InMemorySigner::new(self.signer.clone());
		let tx_block_hash = send_extrinsic(&self.client, &signer, &call, None).await?;
		Ok(tx_block_hash)
	}

	pub async fn dispatch_to_evm(&self, params: EvmParams) -> Result<(), anyhow::Error> {
		let call =
			subxt::dynamic::tx("IsmpDemo", "dispatch_to_evm", vec![evm_params_to_value(&params)]);
		let signer = InMemorySigner::new(self.signer.clone());
		send_extrinsic(&self.client, &signer, &call, None).await?;

		Ok(())
	}

	pub async fn get_request(&self, get_req: GetRequest) -> Result<HashFor<C>, anyhow::Error> {
		let tx = subxt::dynamic::tx(
			"IsmpDemo",
			"get_request",
			vec![get_request_ismp_demo_to_value(&get_req)],
		);
		let signer = InMemorySigner::new(self.signer.clone());
		let tx_block_hash = send_extrinsic(&self.client, &signer, &tx, None).await?;

		Ok(tx_block_hash)
	}

	pub async fn pallet_ismp_demo_events_stream(
		&self,
		count: usize,
		pallet_name: &'static str,
		variant_name: &'static str,
	) -> Result<Vec<EventDetails<C>>, anyhow::Error> {
		let subscription = self.rpc.chain_subscribe_all_heads().await?;
		let client = self.client.clone();
		let stream = subscription.filter_map(move |header_result| {
			let client = client.clone();
			async move {
				let header = header_result.ok()?;

				let hasher = C::Hasher::new(&client.metadata());
				let header_hash = header.hash_with(hasher);

				let events = client.events().at(header_hash).await.ok()?;

				let events = events
					.iter()
					.filter_map(|ev| {
						let ev = ev.ok()?;
						let ev_metadata = ev.event_metadata();
						(ev_metadata.pallet.name() == pallet_name &&
							ev_metadata.variant.name == variant_name)
							.then(|| ev)
					})
					.collect();

				Some(events)
			}
		});

		let mut stream = Box::pin(stream);

		let mut total = 0;
		let mut values = vec![];
		while let Some(mut val) = stream.next().await {
			values.append(&mut val);
			total += values.len();
			if total >= count {
				return Ok(values);
			}
		}
		Err(anyhow::Error::msg("Stream ended"))
	}

	pub async fn runtime_upgrade(&self, code_blob: Vec<u8>) -> anyhow::Result<()> {
		// Set code
		let call = subxt::dynamic::tx("System", "set_code", vec![Value::from_bytes(code_blob)]);

		let tx = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);

		let signer = InMemorySigner::new(self.signer().clone());
		send_extrinsic(&self.client, &signer, &tx, None).await?;

		Ok(())
	}

	pub async fn set_invulnerables(&self, accounts: Vec<C::AccountId>) -> anyhow::Result<()> {
		let call = subxt::dynamic::tx(
			"CollatorSelection",
			"set_invulnerables",
			vec![account_vec_to_value::<C>(&accounts)],
		);
		let tx = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);
		let signer = InMemorySigner::new(self.signer().clone());
		send_extrinsic(&self.client, &signer, &tx, None).await?;

		Ok(())
	}
}
