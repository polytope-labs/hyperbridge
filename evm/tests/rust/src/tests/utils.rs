use polkadot_sdk::*;
use std::{
	collections::{BTreeMap, HashSet},
	path::Path,
};

use crate::{DataOrHash, Mmr};
use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use alloy_sol_types::{SolCall, SolValue};
use ismp_solidity_abi::{
	ecdsa_beefy::Beefy::{IntermediateState, StateCommitment},
	evm_host::EvmHost::{
		requestCommitmentsCall, requestReceiptsCall, responseCommitmentsCall, responseReceiptsCall,
		FeeMetadata, ResponseReceipt,
	},
	handler::{
		handleConsensusCall, handleGetRequestTimeoutsCall, handleGetResponsesCall,
		handlePostRequestTimeoutsCall, handlePostRequestsCall, handlePostResponseTimeoutsCall,
		handlePostResponsesCall, GetResponseMessage, GetTimeoutMessage, PostRequestMessage,
		PostRequestTimeoutMessage, PostResponseMessage, PostResponseTimeoutMessage, Proof,
		StateMachineHeight,
	},
};
use primitive_types::H256;
use revm::{
	context::{Context, TxEnv},
	database::InMemoryDB,
	handler::{ExecuteCommitEvm, MainBuilder, MainContext, MainnetEvm},
	primitives::{hardfork::SpecId, TxKind},
	state::AccountInfo,
};
use sp_core::KeccakHasher;
use sp_trie::{LayoutV0, MemoryDB, StorageProof, TrieDBBuilder, EMPTY_PREFIX};
use trie_db::{HashDB, Recorder, Trie, TrieDBMutBuilder, TrieMut};

// ---------------------------------------------------------------------------
// ABI helpers for non-handler contracts
// ---------------------------------------------------------------------------

alloy_sol_macro::sol! {
	function superApprove(address owner, address spender) external;
	function mint(address to, uint256 amount) external;
	function approve(address spender, uint256 value) external returns (bool);
	function balanceOf(address account) external view returns (uint256);
	function grantMinterRole(address account) external;
	function grantBurnerRole(address account) external;

	// PingModule.setIsmpHost(address, address)
	function setIsmpHost(address hostAddr, address tokenFaucet) external;
}

fn host_manager_set_ismp_host(addr: Address) -> Vec<u8> {
	let selector: [u8; 4] =
		alloy_primitives::keccak256(b"setIsmpHost(address)").0[..4].try_into().unwrap();
	let mut calldata = selector.to_vec();
	calldata.extend_from_slice(&addr.abi_encode());
	calldata
}

// ---------------------------------------------------------------------------
// Constructor param types
// ---------------------------------------------------------------------------

alloy_sol_macro::sol! {
	struct HostManagerParams {
		address admin;
		address host;
	}

	struct PerByteFee {
		bytes32 stateIdHash;
		uint256 perByteFee;
	}

	struct HostParams {
		uint256 defaultTimeout;
		uint256 defaultPerByteFee;
		uint256 stateCommitmentFee;
		address feeToken;
		address admin;
		address handler;
		address hostManager;
		address uniswapV2;
		uint256 unStakingPeriod;
		uint256 challengePeriod;
		address consensusClient;
		uint256[] stateMachines;
		PerByteFee[] perByteFees;
		bytes hyperbridge;
	}
}

// ---------------------------------------------------------------------------
// EVM Runner for revm 27
// ---------------------------------------------------------------------------

type Evm = MainnetEvm<revm::handler::MainnetContext<InMemoryDB>>;

pub struct TestEnv {
	evm: Evm,
	pub sender: Address,
	pub handler: Address,
	pub host: Address,
	pub consensus_client: Address,
	pub fee_token: Address,
	pub test_module: Address,
	pub manager: Address,
}

impl TestEnv {
	fn evm_out_dir() -> std::path::PathBuf {
		let manifest = env!("CARGO_MANIFEST_DIR");
		std::path::PathBuf::from(manifest).join("../../out")
	}

	pub fn evm_out_dir_public(&self) -> std::path::PathBuf {
		Self::evm_out_dir()
	}

	/// Clone the current EVM database — useful for running an isolated inspected call.
	pub fn db_clone(&self) -> InMemoryDB {
		use revm::context_interface::ContextTr;
		self.evm.ctx.db_ref().clone()
	}

	/// Deploy a compiled artifact (with library linking) by name from the out directory.
	pub fn deploy_named(&mut self, out_dir: &std::path::Path, name: &str) -> Address {
		self.deploy_artifact(out_dir, name)
	}

	pub fn new() -> Self {
		let sender = Address::repeat_byte(0x01);

		let mut db = InMemoryDB::default();
		db.insert_account_info(sender, AccountInfo { balance: U256::MAX, ..Default::default() });

		let evm = Context::mainnet()
			.modify_cfg_chained(|cfg| {
				cfg.spec = SpecId::CANCUN;
				cfg.disable_nonce_check = true;
				cfg.limit_contract_code_size = Some(usize::MAX);
				cfg.disable_eip3607 = true;
			})
			.with_db(db)
			.build_mainnet();

		let mut env = TestEnv {
			evm,
			sender,
			handler: Address::ZERO,
			host: Address::ZERO,
			consensus_client: Address::ZERO,
			fee_token: Address::ZERO,
			test_module: Address::ZERO,
			manager: Address::ZERO,
		};

		let out_dir = Self::evm_out_dir();

		// 1. Deploy TestConsensusClientV2
		env.consensus_client = env.deploy_artifact(&out_dir, "TestConsensusClientV2");

		// 2. Deploy HandlerV2 (MerklePatricia library linked automatically)
		env.handler = env.deploy_artifact(&out_dir, "HandlerV2");
		// 3. Deploy FeeToken: constructor(address admin, string name, string symbol)
		let bytecode = load_and_link_artifact(&mut env, &out_dir, "FeeToken");
		let constructor_args =
			(env.sender, "HyperUSD".to_string(), "USD.h".to_string()).abi_encode_params();
		env.fee_token = env.deploy_raw([bytecode, constructor_args].concat());

		// 4. Deploy HostManager: constructor(HostManagerParams)
		let bytecode = load_and_link_artifact(&mut env, &out_dir, "HostManager");
		let params = HostManagerParams { admin: env.sender, host: Address::ZERO };
		let constructor_args = SolValue::abi_encode(&params);
		env.manager = env.deploy_raw([bytecode, constructor_args].concat());

		// 5. Deploy TestHost: constructor(HostParams)
		let bytecode = load_and_link_artifact(&mut env, &out_dir, "TestHost");
		let host_params = HostParams {
			defaultTimeout: U256::ZERO,
			defaultPerByteFee: U256::from(1_000_000_000_000_000_000u128),
			stateCommitmentFee: U256::from(10u128) * U256::from(10u128.pow(18)),
			feeToken: env.fee_token,
			admin: env.sender,
			handler: env.handler,
			hostManager: env.manager,
			uniswapV2: Address::ZERO,
			unStakingPeriod: U256::from(21u64 * 60 * 60 * 24),
			challengePeriod: U256::ZERO,
			consensusClient: env.consensus_client,
			stateMachines: vec![U256::from(2000)],
			perByteFees: vec![],
			hyperbridge: Bytes::from(b"KUSAMA-2000".to_vec()),
		};
		let constructor_args = SolValue::abi_encode(&host_params);
		env.host = env.deploy_raw([bytecode, constructor_args].concat());

		// 6. Deploy PingModule: constructor(address admin)
		let bytecode = load_and_link_artifact(&mut env, &out_dir, "PingModule");
		let constructor_args = (env.sender,).abi_encode_params();
		env.test_module = env.deploy_raw([bytecode, constructor_args].concat());

		// 7. Configure: setIsmpHost on PingModule (needs warped timestamp)
		env.evm.ctx.block.timestamp = U256::from(100_000);
		env.call(
			env.test_module,
			setIsmpHostCall { hostAddr: env.host, tokenFaucet: Address::ZERO }.abi_encode(),
		);
		env.evm.ctx.block.timestamp = U256::from(1);

		// 8. Configure: setIsmpHost on HostManager
		env.call(env.manager, host_manager_set_ismp_host(env.host));

		// 9. Token approvals
		env.call(
			env.fee_token,
			superApproveCall { owner: env.sender, spender: env.host }.abi_encode(),
		);
		env.call(
			env.fee_token,
			superApproveCall { owner: env.test_module, spender: env.host }.abi_encode(),
		);
		env.call(
			env.fee_token,
			superApproveCall { owner: env.sender, spender: env.test_module }.abi_encode(),
		);

		// 10. Grant minter/burner roles to sender
		env.call(env.fee_token, grantMinterRoleCall { account: env.sender }.abi_encode());
		env.call(env.fee_token, grantBurnerRoleCall { account: env.sender }.abi_encode());

		env
	}

	fn deploy_artifact(&mut self, out_dir: &Path, contract_name: &str) -> Address {
		let bytecode = load_and_link_artifact(self, out_dir, contract_name);
		self.deploy_raw(bytecode)
	}

	fn deploy_raw(&mut self, bytecode: Vec<u8>) -> Address {
		let tx = TxEnv::builder()
			.caller(self.sender)
			.kind(TxKind::Create)
			.data(Bytes::from(bytecode))
			.value(U256::ZERO)
			.gas_limit(30_000_000)
			.build_fill();

		let result = self.evm.transact_commit(tx).unwrap();

		match result {
			ref r if r.is_success() => r.created_address().expect("no address from CREATE"),
			other => panic!("deployment failed: {other:?}"),
		}
	}

	pub fn call(&mut self, to: Address, calldata: Vec<u8>) -> Vec<u8> {
		let tx = TxEnv::builder()
			.caller(self.sender)
			.kind(TxKind::Call(to))
			.data(Bytes::from(calldata))
			.value(U256::ZERO)
			.gas_limit(30_000_000)
			.build_fill();

		let result = self.evm.transact_commit(tx).unwrap();

		match result {
			revm::context_interface::result::ExecutionResult::Success {
				output: revm::context_interface::result::Output::Call(data),
				..
			} => data.to_vec(),
			other => panic!("call to {to:?} failed: {other:?}"),
		}
	}

	pub fn call_reverts(&mut self, to: Address, calldata: Vec<u8>) -> bool {
		let tx = TxEnv::builder()
			.caller(self.sender)
			.kind(TxKind::Call(to))
			.data(Bytes::from(calldata))
			.value(U256::ZERO)
			.gas_limit(30_000_000)
			.build_fill();

		let result = self.evm.transact_commit(tx).unwrap();
		matches!(result, revm::context_interface::result::ExecutionResult::Revert { .. })
	}

	/// Call a contract with an explicit caller (impersonating that address)
	pub fn call_as(&mut self, caller: Address, to: Address, calldata: Vec<u8>) -> Vec<u8> {
		// Ensure the caller has a balance so revm accepts the tx
		{
			use revm::context_interface::ContextTr;
			let acct = self.evm.ctx.db_mut().load_account(caller).unwrap();
			if acct.info.balance < U256::from(u64::MAX) {
				acct.info.balance = U256::MAX;
			}
		}
		let tx = TxEnv::builder()
			.caller(caller)
			.kind(TxKind::Call(to))
			.data(Bytes::from(calldata))
			.value(U256::ZERO)
			.gas_limit(30_000_000)
			.build_fill();

		let result = self.evm.transact_commit(tx).unwrap();
		match result {
			revm::context_interface::result::ExecutionResult::Success {
				output: revm::context_interface::result::Output::Call(data),
				..
			} => data.to_vec(),
			other => panic!("call_as {caller:?} to {to:?} failed: {other:?}"),
		}
	}

	/// Call a contract with an explicit caller, returning Err on revert with its output.
	pub fn call_as_may_revert(
		&mut self,
		caller: Address,
		to: Address,
		calldata: Vec<u8>,
	) -> Result<Vec<u8>, Vec<u8>> {
		{
			use revm::context_interface::ContextTr;
			let acct = self.evm.ctx.db_mut().load_account(caller).unwrap();
			if acct.info.balance < U256::from(u64::MAX) {
				acct.info.balance = U256::MAX;
			}
		}
		let tx = TxEnv::builder()
			.caller(caller)
			.kind(TxKind::Call(to))
			.data(Bytes::from(calldata))
			.value(U256::ZERO)
			.gas_limit(30_000_000)
			.build_fill();

		let result = self.evm.transact_commit(tx).unwrap();
		match result {
			revm::context_interface::result::ExecutionResult::Success {
				output: revm::context_interface::result::Output::Call(data),
				..
			} => Ok(data.to_vec()),
			revm::context_interface::result::ExecutionResult::Revert { output, .. } =>
				Err(output.to_vec()),
			other => panic!("call_as_may_revert unexpected result: {other:?}"),
		}
	}

	pub fn warp(&mut self, delta: u64) {
		self.evm.ctx.block.timestamp += U256::from(delta);
	}

	pub fn warp_to(&mut self, timestamp: U256) {
		self.evm.ctx.block.timestamp = timestamp;
	}

	pub fn block_timestamp(&self) -> u64 {
		self.evm.ctx.block.timestamp.to::<u64>()
	}

	pub fn encode_consensus_proof(
		state_machine_id: U256,
		height: U256,
		timestamp: U256,
		overlay_root: [u8; 32],
		state_root: [u8; 32],
		next_authority_set_id: U256,
	) -> Vec<u8> {
		let intermediate = IntermediateState {
			stateMachineId: state_machine_id,
			height,
			commitment: StateCommitment {
				timestamp,
				overlayRoot: FixedBytes(overlay_root),
				stateRoot: FixedBytes(state_root),
			},
		};
		(intermediate, next_authority_set_id).abi_encode_params()
	}

	// -- Handler convenience methods --

	pub fn handle_consensus(&mut self, consensus_proof: Vec<u8>) {
		self.call(
			self.handler,
			handleConsensusCall { host: self.host, proof: consensus_proof.into() }.abi_encode(),
		);
	}

	pub fn handle_post_requests(&mut self, message: PostRequestMessage) {
		self.call(
			self.handler,
			handlePostRequestsCall { host: self.host, request: message }.abi_encode(),
		);
	}

	pub fn handle_post_responses(&mut self, message: PostResponseMessage) {
		self.call(
			self.handler,
			handlePostResponsesCall { host: self.host, response: message }.abi_encode(),
		);
	}

	pub fn handle_get_responses(&mut self, message: GetResponseMessage) {
		self.call(self.handler, handleGetResponsesCall { host: self.host, message }.abi_encode());
	}

	pub fn handle_post_request_timeouts(&mut self, message: PostRequestTimeoutMessage) {
		self.call(
			self.handler,
			handlePostRequestTimeoutsCall { host: self.host, message }.abi_encode(),
		);
	}

	pub fn handle_post_response_timeouts(&mut self, message: PostResponseTimeoutMessage) {
		self.call(
			self.handler,
			handlePostResponseTimeoutsCall { host: self.host, message }.abi_encode(),
		);
	}

	pub fn handle_get_request_timeouts(&mut self, message: GetTimeoutMessage) {
		self.call(
			self.handler,
			handleGetRequestTimeoutsCall { host: self.host, message }.abi_encode(),
		);
	}

	pub fn request_receipt(&mut self, commitment: [u8; 32]) -> Address {
		let result = self.call(
			self.host,
			requestReceiptsCall { commitment: FixedBytes(commitment) }.abi_encode(),
		);
		<requestReceiptsCall as SolCall>::abi_decode_returns(&result).unwrap()
	}

	pub fn response_receipt(&mut self, commitment: [u8; 32]) -> ResponseReceipt {
		let result = self.call(
			self.host,
			responseReceiptsCall { commitment: FixedBytes(commitment) }.abi_encode(),
		);
		<responseReceiptsCall as SolCall>::abi_decode_returns(&result).unwrap()
	}

	pub fn request_commitment(&mut self, commitment: [u8; 32]) -> FeeMetadata {
		let result = self.call(
			self.host,
			requestCommitmentsCall { commitment: FixedBytes(commitment) }.abi_encode(),
		);
		<requestCommitmentsCall as SolCall>::abi_decode_returns(&result).unwrap()
	}

	pub fn response_commitment(&mut self, commitment: [u8; 32]) -> FeeMetadata {
		let result = self.call(
			self.host,
			responseCommitmentsCall { commitment: FixedBytes(commitment) }.abi_encode(),
		);
		<responseCommitmentsCall as SolCall>::abi_decode_returns(&result).unwrap()
	}

	pub fn dispatch_post_request(
		&mut self,
		request: ismp_solidity_abi::evm_host::EvmHost::PostRequest,
	) {
		// Convert EvmHost::PostRequest to PingModule::PostRequest via ABI encoding
		let encoded = SolValue::abi_encode(&request);
		let ping_request =
			<ismp_solidity_abi::ping_module::PingModule::PostRequest as SolValue>::abi_decode(
				&encoded,
			)
			.unwrap();
		let calldata =
			ismp_solidity_abi::ping_module::PingModule::dispatch_0Call { request: ping_request }
				.abi_encode();
		self.call(self.test_module, calldata);
	}

	pub fn dispatch_get_request(
		&mut self,
		request: ismp_solidity_abi::evm_host::EvmHost::GetRequest,
	) {
		let encoded = SolValue::abi_encode(&request);
		let ping_request =
			<ismp_solidity_abi::ping_module::PingModule::GetRequest as SolValue>::abi_decode(
				&encoded,
			)
			.unwrap();
		let calldata =
			ismp_solidity_abi::ping_module::PingModule::dispatch_1Call { request: ping_request }
				.abi_encode();
		self.call(self.test_module, calldata);
	}

	pub fn dispatch_post_response(
		&mut self,
		response: ismp_solidity_abi::evm_host::EvmHost::PostResponse,
	) {
		let encoded = SolValue::abi_encode(&response);
		let ping_response =
			<ismp_solidity_abi::ping_module::PingModule::PostResponse as SolValue>::abi_decode(
				&encoded,
			)
			.unwrap();
		let calldata = ismp_solidity_abi::ping_module::PingModule::dispatchPostResponseCall {
			response: ping_response,
		}
		.abi_encode();
		self.call(self.test_module, calldata);
	}

	pub fn mint_fee_token(&mut self, to: Address, amount: U256) {
		self.call(self.fee_token, mintCall { to, amount }.abi_encode());
	}

	pub fn approve_fee_token(&mut self, spender: Address, value: U256) {
		self.call(self.fee_token, approveCall { spender, value }.abi_encode());
	}

	pub fn fee_token_balance(&mut self, account: Address) -> U256 {
		let result = self.call(self.fee_token, balanceOfCall { account }.abi_encode());
		<balanceOfCall as SolCall>::abi_decode_returns(&result).unwrap()
	}
}

// ---------------------------------------------------------------------------
// Bytecode loading with automatic library linking
// ---------------------------------------------------------------------------

fn load_and_link_artifact(runner: &mut TestEnv, out_dir: &Path, artifact_name: &str) -> Vec<u8> {
	for entry in std::fs::read_dir(out_dir).unwrap() {
		let entry = entry.unwrap();
		if entry.file_type().unwrap().is_dir() {
			let json_path = entry.path().join(format!("{artifact_name}.json"));
			if json_path.exists() {
				let content = std::fs::read_to_string(&json_path).unwrap();
				let artifact: json::Value = json::from_str(&content).unwrap();
				let mut bytecode_hex = artifact["bytecode"]["object"]
					.as_str()
					.expect("missing bytecode.object")
					.to_string();

				if let Some(link_refs) = artifact["bytecode"]["linkReferences"].as_object() {
					for (_source_file, libs) in link_refs {
						for (lib_name, offsets) in libs.as_object().unwrap() {
							let lib_bytecode = load_and_link_artifact(runner, out_dir, lib_name);
							let lib_addr = runner.deploy_raw(lib_bytecode);
							let addr_hex = hex::encode(lib_addr.as_slice());

							for offset_info in offsets.as_array().unwrap() {
								let start = offset_info["start"].as_u64().unwrap() as usize;
								let length = offset_info["length"].as_u64().unwrap() as usize;
								assert_eq!(length, 20);

								let hex_start = if bytecode_hex.starts_with("0x") {
									2 + start * 2
								} else {
									start * 2
								};
								let hex_end = hex_start + length * 2;
								bytecode_hex.replace_range(hex_start..hex_end, &addr_hex);
							}
						}
					}
				}

				let hex_str = bytecode_hex.strip_prefix("0x").unwrap_or(&bytecode_hex);
				return hex::decode(hex_str).expect("invalid hex in bytecode after linking");
			}
		}
	}
	panic!("Artifact for '{artifact_name}' not found in {out_dir:?}");
}

// ---------------------------------------------------------------------------
// Type conversion helpers
// ---------------------------------------------------------------------------

pub fn to_handler_get_response(
	response: ismp::router::GetResponse,
) -> ismp_solidity_abi::handler::GetResponse {
	let evm_response: ismp_solidity_abi::evm_host::EvmHost::GetResponse = response.into();
	let encoded = SolValue::abi_encode(&evm_response);
	<ismp_solidity_abi::handler::GetResponse as SolValue>::abi_decode(&encoded).unwrap()
}

pub fn to_handler_get_request(
	request: ismp_solidity_abi::evm_host::EvmHost::GetRequest,
) -> ismp_solidity_abi::handler::GetRequest {
	let encoded = SolValue::abi_encode(&request);
	<ismp_solidity_abi::handler::GetRequest as SolValue>::abi_decode(&encoded).unwrap()
}

pub fn to_handler_post_request(
	request: ismp_solidity_abi::evm_host::EvmHost::PostRequest,
) -> ismp_solidity_abi::handler::PostRequest {
	let encoded = SolValue::abi_encode(&request);
	<ismp_solidity_abi::handler::PostRequest as SolValue>::abi_decode(&encoded).unwrap()
}

// ---------------------------------------------------------------------------
// MMR and trie proof utilities
// ---------------------------------------------------------------------------

pub fn initialize_mmr_tree(
	leaf: DataOrHash,
	block_height: u64,
) -> Result<([u8; 32], Proof), anyhow::Error> {
	let mut mmr = Mmr::default();
	for _ in 0..30 {
		mmr.push(DataOrHash::Hash(H256::random()))?;
	}
	let pos = mmr.push(leaf)?;
	for _ in 0..30 {
		mmr.push(DataOrHash::Hash(H256::random()))?;
	}

	let proof = mmr.gen_proof(vec![pos])?;
	let root = mmr.get_root()?.hash().0;
	let multiproof: Vec<FixedBytes<32>> =
		proof.proof_items().iter().map(|h| FixedBytes(h.hash().0)).collect();
	let height =
		StateMachineHeight { stateMachineId: U256::from(2000), height: U256::from(block_height) };

	Ok((root, Proof { height, multiproof, leafCount: U256::from(61) }))
}

pub fn generate_non_membership_proof(
	prefix: Vec<u8>,
	keys: Vec<Vec<u8>>,
	insert_keys: bool,
) -> (H256, Vec<Vec<u8>>) {
	let mut entries: Vec<_> = (1..50)
		.map(|_| {
			let mut key = prefix.clone();
			key.extend_from_slice(&H256::random().0);
			(key, H256::random().0.to_vec())
		})
		.collect();

	if insert_keys {
		entries.extend(keys.iter().map(|key| (key.clone(), H256::random().0.to_vec())));
	}

	let (db, root) = {
		let mut db = <MemoryDB<KeccakHasher>>::default();
		let mut root = Default::default();
		{
			let mut trie =
				TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
			for (key, value) in &entries {
				trie.insert(key, value).unwrap();
			}
		}
		(db, root)
	};

	let proof = {
		let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
		let trie_db = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
			.with_recorder(&mut recorder)
			.build();
		for key in &keys {
			let _ = trie_db.get(key).unwrap();
		}
		recorder
			.drain()
			.into_iter()
			.map(|f| f.data)
			.collect::<HashSet<_>>()
			.into_iter()
			.collect()
	};

	(root, proof)
}

pub fn read_proof_check<I>(
	root: &H256,
	proof: StorageProof,
	keys: I,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, ()>
where
	I: IntoIterator,
	I::Item: AsRef<[u8]>,
{
	let db = proof.into_memory_db::<KeccakHasher>();
	if !db.contains(root, EMPTY_PREFIX) {
		return Err(());
	}
	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, root).build();
	let mut result = BTreeMap::new();
	for key in keys {
		let value = trie.get(key.as_ref()).map_err(|_| ())?;
		result.insert(key.as_ref().to_vec(), value);
	}
	Ok(result)
}
