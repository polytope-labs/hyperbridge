// Copyright (C) Polytope Labs Ltd.
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

//! Substrate EVM State Machine client implementation

use crate::{prelude::*, req_res_commitment_key, req_res_receipt_keys};
use alloc::collections::BTreeMap;
use codec::{Decode, Encode};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::Proof,
	router::RequestResponse,
};
use pallet_ismp_host_executive::EvmHosts;
use polkadot_sdk::*;
use primitive_types::H160;
use sp_core::{hashing, storage::ChildInfo, H256};
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};

/// Proof structure for Substrate EVM verification
#[derive(Decode, Encode)]
pub struct SubstrateEvmProof {
	/// Proof of the Contract AccountInfo and the Child Trie Root in the Main State Trie
	pub main_proof: Vec<Vec<u8>>,
	/// Proof of the Storage Slots in the Contract's Child Trie
	pub child_proof: Vec<Vec<u8>>,
}

/// Substrate EVM State machine client
pub struct SubstrateEvmStateMachine<H: IsmpHost, T: pallet_ismp_host_executive::Config>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default
	for SubstrateEvmStateMachine<H, T>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for SubstrateEvmStateMachine<H, T> {
	fn clone(&self) -> Self {
		Self::default()
	}
}

impl<H: IsmpHost + Send + Sync, T: pallet_ismp_host_executive::Config> StateMachineClient
	for SubstrateEvmStateMachine<H, T>
{
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		item: RequestResponse,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;

		let proof: SubstrateEvmProof = Decode::decode(&mut &proof.proof[..])
			.map_err(|e| Error::Custom(format!("Failed to decode proof: {:?}", e)))?;

		let state_root = H256::from_slice(&root.state_root[..]);

		// verify contact info in main trie first to get Trie Id
		let contract_info_key = contract_info_key(contract_address);
		let trie_id =
			fetch_trie_id_from_main_proof::<H>(&proof.main_proof, state_root, &contract_info_key)?;

		let child_root =
			fetch_child_root_from_main_proof::<H>(&proof.main_proof, state_root, &trie_id)?;

		// verify storage slots in child trie
		let keys = req_res_commitment_key::<H>(item);

		// convert evm keys to child trie keys
		let storage_keys: Vec<Vec<u8>> =
			keys.into_iter().map(|k| hashing::blake2_256(&k).to_vec()).collect();

		verify_child_trie_membership::<H>(child_root, &proof.child_proof, storage_keys)
	}

	fn receipts_state_trie_key(&self, request: RequestResponse) -> Vec<Vec<u8>> {
		req_res_receipt_keys::<H>(request)
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;

		let proof: SubstrateEvmProof = Decode::decode(&mut &proof.proof[..])
			.map_err(|e| Error::Custom(format!("Failed to decode proof: {:?}", e)))?;

		let state_root = H256::from_slice(&root.state_root[..]);

		let contract_info_key = contract_info_key(contract_address);
		let trie_id =
			fetch_trie_id_from_main_proof::<H>(&proof.main_proof, state_root, &contract_info_key)?;

		let child_root =
			fetch_child_root_from_main_proof::<H>(&proof.main_proof, state_root, &trie_id)?;

		let storage_keys: Vec<Vec<u8>> =
			keys.iter().map(|k| hashing::blake2_256(k).to_vec()).collect();

		let values = verify_child_trie_values::<H>(child_root, &proof.child_proof, storage_keys)?;

		let mut map = BTreeMap::new();
		for (key, value) in keys.into_iter().zip(values.into_iter()) {
			map.insert(key, value);
		}

		Ok(map)
	}
}

fn contract_info_key(address: H160) -> Vec<u8> {
	let mut key = Vec::new();
	key.extend_from_slice(&hashing::twox_128(b"Revive"));
	key.extend_from_slice(&hashing::twox_128(b"AccountInfoOf"));
	key.extend_from_slice(address.as_bytes());
	key
}

fn fetch_trie_id_from_main_proof<H: IsmpHost>(
	proof: &[Vec<u8>],
	root: H256,
	key: &[u8],
) -> Result<Vec<u8>, Error> {
	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	let val = trie
		.get(key)
		.map_err(|e| Error::Custom(format!("Trie error: {:?}", e)))?
		.ok_or_else(|| Error::Custom("Contract Info not found in main trie".to_string()))?;

	// Decodes AccountInfo to get trie_id
	// AccountInfo { account_type: enum { Contract(ContractInfo { trie_id, ... }) = 0, ... }, ... }
	let mut input = &val[..];
	let variant_index = u8::decode(&mut input)
		.map_err(|_| Error::Custom("Failed to decode AccountInfo variant".to_string()))?;

	if variant_index != 0 {
		return Err(Error::Custom("Account is not a contract".to_string()));
	}

	let trie_id = Vec::<u8>::decode(&mut input)
		.map_err(|_| Error::Custom("Failed to decode trie_id".to_string()))?;

	Ok(trie_id)
}

fn fetch_child_root_from_main_proof<H: IsmpHost>(
	proof: &[Vec<u8>],
	root: H256,
	trie_id: &[u8],
) -> Result<H256, Error> {
	let child_info = ChildInfo::new_default(trie_id);
	let key = child_info.prefixed_storage_key();

	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	let val = trie
		.get(&key)
		.map_err(|e| Error::Custom(format!("Trie error: {:?}", e)))?
		.ok_or_else(|| Error::Custom("Child Trie Root not found in main trie".to_string()))?;

	let child_root = H256::decode(&mut &val[..])
		.map_err(|_| Error::Custom("Failed to decode child root".to_string()))?;

	Ok(child_root)
}

fn verify_child_trie_membership<H: IsmpHost>(
	root: H256,
	proof: &[Vec<u8>],
	keys: Vec<Vec<u8>>,
) -> Result<(), Error> {
	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	for key in keys {
		let val = trie
			.get(&key)
			.map_err(|e| Error::Custom(format!("Child Trie error: {:?}", e)))?;
		if val.is_none() {
			return Err(Error::Custom(format!("Key {:?} not found in child trie", key)));
		}
	}
	Ok(())
}

fn verify_child_trie_values<H: IsmpHost>(
	root: H256,
	proof: &[Vec<u8>],
	keys: Vec<Vec<u8>>,
) -> Result<Vec<Option<Vec<u8>>>, Error> {
	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	let mut values = Vec::new();
	for key in keys {
		let val = trie
			.get(&key)
			.map_err(|e| Error::Custom(format!("Child Trie error: {:?}", e)))?;
		values.push(val);
	}
	Ok(values)
}

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::host::StateMachine;
	use primitive_types::U256;
	use serde::{Deserialize, Serialize};
	use serde_json::json;
	use std::time::Duration;
	use ismp::consensus::{ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineHeight, StateMachineId};
	use ismp::messaging::Keccak256;
	use ismp::router::{IsmpRouter, PostResponse, Request, Response};

	struct MockHost;

	impl Keccak256 for MockHost {
		fn keccak256(bytes: &[u8]) -> H256
		where
			Self: Sized
		{
			todo!()
		}
	}

	impl IsmpHost for MockHost {
		fn host_state_machine(&self) -> StateMachine {
			todo!()
		}

		fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, Error> {
			todo!()
		}

		fn state_machine_commitment(&self, height: StateMachineHeight) -> Result<StateCommitment, Error> {
			todo!()
		}

		fn consensus_update_time(&self, consensus_state_id: ConsensusStateId) -> Result<Duration, Error> {
			todo!()
		}

		fn state_machine_update_time(&self, state_machine_height: StateMachineHeight) -> Result<Duration, Error> {
			todo!()
		}

		fn consensus_client_id(&self, consensus_state_id: ConsensusStateId) -> Option<ConsensusClientId> {
			todo!()
		}

		fn consensus_state(&self, consensus_state_id: ConsensusStateId) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn timestamp(&self) -> Duration {
			todo!()
		}

		fn is_consensus_client_frozen(&self, consensus_state_id: ConsensusStateId) -> Result<(), Error> {
			todo!()
		}

		fn request_commitment(&self, req: H256) -> Result<(), Error> {
			todo!()
		}

		fn response_commitment(&self, req: H256) -> Result<(), Error> {
			todo!()
		}

		fn next_nonce(&self) -> u64 {
			todo!()
		}

		fn request_receipt(&self, req: &Request) -> Option<()> {
			todo!()
		}

		fn response_receipt(&self, res: &Response) -> Option<()> {
			todo!()
		}

		fn store_consensus_state_id(&self, consensus_state_id: ConsensusStateId, client_id: ConsensusClientId) -> Result<(), Error> {
			todo!()
		}

		fn store_consensus_state(&self, consensus_state_id: ConsensusStateId, consensus_state: Vec<u8>) -> Result<(), Error> {
			todo!()
		}

		fn store_unbonding_period(&self, consensus_state_id: ConsensusStateId, period: u64) -> Result<(), Error> {
			todo!()
		}

		fn store_consensus_update_time(&self, consensus_state_id: ConsensusStateId, timestamp: Duration) -> Result<(), Error> {
			todo!()
		}

		fn store_state_machine_update_time(&self, state_machine_height: StateMachineHeight, timestamp: Duration) -> Result<(), Error> {
			todo!()
		}

		fn store_state_machine_commitment(&self, height: StateMachineHeight, state: StateCommitment) -> Result<(), Error> {
			todo!()
		}

		fn delete_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
			todo!()
		}

		fn freeze_consensus_client(&self, consensus_state_id: ConsensusStateId) -> Result<(), Error> {
			todo!()
		}

		fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
			todo!()
		}

		fn delete_request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn delete_response_commitment(&self, res: &PostResponse) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn delete_request_receipt(&self, req: &Request) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn delete_response_receipt(&self, res: &Response) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn store_request_receipt(&self, req: &Request, signer: &Vec<u8>) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn store_response_receipt(&self, req: &Response, signer: &Vec<u8>) -> Result<Vec<u8>, Error> {
			todo!()
		}

		fn store_request_commitment(&self, req: &Request, meta: Vec<u8>) -> Result<(), Error> {
			todo!()
		}

		fn store_response_commitment(&self, res: &PostResponse, meta: Vec<u8>) -> Result<(), Error> {
			todo!()
		}

		fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
			todo!()
		}

		fn challenge_period(&self, state_machine: StateMachineId) -> Option<Duration> {
			todo!()
		}

		fn store_challenge_period(&self, state_machine: StateMachineId, period: u64) -> Result<(), Error> {
			todo!()
		}

		fn allowed_proxy(&self) -> Option<StateMachine> {
			todo!()
		}

		fn unbonding_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration> {
			todo!()
		}

		fn ismp_router(&self) -> Box<dyn IsmpRouter> {
			todo!()
		}

		fn previous_commitment_height(&self, id: StateMachineId) -> Option<u64> {
			todo!()
		}
	}

	fn get_rpc_url() -> String {
		std::env::var("SUBSTRATE_RPC_URL")
			.unwrap_or_else(|_| "https://asset-hub-westend-rpc.dwellir.com".to_string())
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct RpcResponse<T> {
		result: Option<T>,
		error: Option<serde_json::Value>,
		id: u64,
		jsonrpc: String,
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct ReadProof {
		proof: Vec<String>,
	}

	async fn rpc_request<T: serde::de::DeserializeOwned>(
		method: &str,
		params: Vec<serde_json::Value>,
	) -> Result<T, Box<dyn std::error::Error>> {
		let client = reqwest::Client::builder()
			.http1_only()
			.build()?;
		let url = get_rpc_url();

		let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

		let body_str = serde_json::to_string(&body)?;

		let resp_bytes = client
			.post(&url)
			.header("Content-Type", "application/json")
			.body(body_str)
			.send()
			.await?
			.bytes()
			.await?;

		let resp: RpcResponse<T> = serde_json::from_slice(&resp_bytes)?;

		if let Some(err) = resp.error {
			return Err(format!("RPC Error: {:?}", err).into());
		}

		resp.result.ok_or_else(|| "No result in response".into())
	}

	#[ignore]
	#[tokio::test]
	async fn test_verify_revive_state_proof() -> Result<(), Box<dyn std::error::Error>> {
		tracing_subscriber::fmt::try_init();

		let contract_hex = std::env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS must be set");
		let contract_address = H160::from_slice(&hex::decode(contract_hex.trim_start_matches("0x"))?);

		println!("Fetching block hash...");
		let block_hash: String = rpc_request("chain_getBlockHash", vec![]).await?;
		println!("Testing at block: {}", block_hash);
		let block_hash_h256 = H256::from_slice(&hex::decode(block_hash.trim_start_matches("0x"))?);

		println!("Fetching block header...");
		let header: serde_json::Value = rpc_request("chain_getHeader", vec![json!(block_hash)]).await?;
		let state_root_hex = header["stateRoot"].as_str().expect("stateRoot missing");
		let state_root = H256::from_slice(&hex::decode(state_root_hex.trim_start_matches("0x"))?);

		let contract_info_key = contract_info_key(contract_address);
		let storage_key_hex = format!("0x{}", hex::encode(&contract_info_key));

		println!("Fetching account info storage...");
		let account_info_hex: Option<String> = rpc_request(
			"state_getStorage",
			vec![json!(storage_key_hex), json!(block_hash)]
		).await?;

		let account_info_bytes = hex::decode(
			account_info_hex.ok_or("AccountInfo not found - is this a valid contract?")?
				.trim_start_matches("0x")
		)?;

		let mut input = &account_info_bytes[..];
		let variant = u8::decode(&mut input)?;
		if variant != 0 {
			return Err("Account is not a contract".into());
		}
		let trie_id = Vec::<u8>::decode(&mut input)?;

		let child_info = ChildInfo::new_default(&trie_id);
		let child_root_key = child_info.prefixed_storage_key();
		let child_storage_key_hex = format!("0x{}", hex::encode(child_info.prefixed_storage_key().into_inner()));

		println!("Fetching a valid existing key from child trie to use for verification...");
		let keys_paged: Vec<String> = rpc_request(
			"childstate_getKeysPaged",
			vec![
				json!(child_storage_key_hex),
				json!("0x"),
				json!(1),
				json!(null),
				json!(block_hash)
			]
		).await?;

		let active_key_hex = keys_paged.first().ok_or("No keys found in contract child trie")?;
		println!("Found active key: {}", active_key_hex);
		let active_key = hex::decode(active_key_hex.trim_start_matches("0x"))?;

		let main_keys = vec![
			contract_info_key.clone(),
			child_root_key.into_inner(),
		];
		let main_keys_hex: Vec<String> = main_keys.iter().map(|k| format!("0x{}", hex::encode(k))).collect();

		println!("Fetching main proof...");
		let main_read_proof: ReadProof = rpc_request(
			"state_getReadProof",
			vec![json!(main_keys_hex), json!(block_hash)]
		).await?;

		let main_proof_bytes: Vec<Vec<u8>> = main_read_proof.proof
			.iter()
			.map(|p| hex::decode(p.trim_start_matches("0x")).unwrap())
			.collect();


		let child_keys_hex = vec![active_key_hex.clone()];

		println!("Fetching child proof...");
		let child_read_proof: ReadProof = rpc_request(
			"state_getChildReadProof",
			vec![
				json!(child_storage_key_hex),
				json!(child_keys_hex),
				json!(block_hash)
			]
		).await?;

		let child_proof_bytes: Vec<Vec<u8>> = child_read_proof.proof
			.iter()
			.map(|p| hex::decode(p.trim_start_matches("0x")).unwrap())
			.collect();

		println!("Proofs fetched. Verifying...");

		let verified_trie_id = fetch_trie_id_from_main_proof::<MockHost>(
			&main_proof_bytes,
			state_root,
			&contract_info_key
		)?;
		assert_eq!(verified_trie_id, trie_id, "Trie ID mismatch");
		println!("Main Proof Verified: Trie ID matches");

		let verified_child_root = fetch_child_root_from_main_proof::<MockHost>(
			&main_proof_bytes,
			state_root,
			&verified_trie_id
		)?;
		println!("Main Proof Verified: Child Root gotten: {:?}", verified_child_root);

		let values = verify_child_trie_values::<MockHost>(
			verified_child_root,
			&child_proof_bytes,
			vec![active_key]
		)?;

		assert_eq!(values.len(), 1);
		match &values[0] {
			Some(val) => println!("Child Proof Verified: Value found with length {}", val.len()),
			None => return Err("Child Proof Verified: Value is None but key was requested!".into()),
		}

		Ok(())
	}
}
