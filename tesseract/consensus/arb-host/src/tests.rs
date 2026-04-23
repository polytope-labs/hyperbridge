use alloy::{eips::BlockId, providers::Provider, sol_types::SolEvent};
use alloy_primitives::{keccak256, Address, Bytes, U256};
use alloy_rlp::Decodable;
use arbitrum_verifier::verify_arbitrum_bold;
use geth_primitives::Header;
use tesseract_evm::EvmConfig;
use tesseract_primitives::Hasher;

use crate::{abi::IRollupBold, ArbConfig, ArbHost, HostConfig};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use hex_literal::hex;
use ismp::host::StateMachine;
use primitive_types::{H160, H256};

const ROLLUP_CORE: [u8; 20] = hex!("d80810638dbDF9081b72C1B33c65375e807281C8");

/// Placeholder secp256k1 key for read-only tests. `EvmClient::new` requires a valid signer to
/// construct, but none of the consensus-verification flow this file exercises actually signs
/// or sends transactions.
const DUMMY_SIGNING_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
pub struct Block {
	header: Header,
	transactions: Vec<Transaction>,
	uncles: Vec<Header>,
}

// // LegacyTx is the transaction data of regular Ethereum transactions.
// type LegacyTx struct {
//     Nonce    uint64          // nonce of sender account
//     GasPrice *big.Int        // wei per gas
//     Gas      uint64          // gas limit
//     To       *common.Address `rlp:"nil"` // nil means contract creation
//     Value    *big.Int        // wei amount
//     Data     []byte          // contract invocation input data
//     V, R, S  *big.Int        // signature values
// }

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
pub struct Transaction {
	pub nonce: u64,
	pub gas_price: U256,
	/// Gas amount
	pub gas: u64,
	/// Recipient (None when contract creation)
	pub to: Address,
	/// Transferred value
	pub value: U256,

	/// Input data
	pub data: Bytes,
	/// ECDSA recovery id
	pub v: U256,

	/// ECDSA signature r
	pub r: U256,

	/// ECDSA signature s
	pub s: U256,
}

/// End-to-end verification of a BoLD `AssertionCreated` event on Arbitrum One (mainnet)
/// using the full host-side proof pipeline.
///
/// The event's hex encoding is the *non-indexed* ABI-encoded data (`AssertionInputs` is an
/// all-static nested tuple, so no offsets are used). The two indexed topics — `assertionHash`
/// and `parentAssertionHash` — are derived here by hashing the exact tuple the rollup-core
/// indexes under, exactly matching the verifier's own `compute_assertion_hash`:
///
/// ```text
/// parentAssertionHash = keccak256(prevPrevAssertionHash ∥ keccak256(abi.encode(beforeState)) ∥ sequencerBatchAcc)
/// assertionHash       = keccak256(parentAssertionHash     ∥ keccak256(abi.encode(afterState))  ∥ afterInboxBatchAcc)
/// ```
///
/// Rollup proxy: `0x4DCeB440657f21083db8aDd07665f8ddBe1DCfc0` on L1 Ethereum mainnet. Requires
/// `MAINNET_RPC_URL` (L1) and `ARB_MAINNET_RPC_URL` (L2 Arbitrum One) in the environment.
#[tokio::test]
#[ignore]
async fn test_arbitrum_bold_assertion_verification() {
	dotenv::dotenv().ok();
	let l1_url = std::env::var("MAINNET_RPC_URL")
		.expect("MAINNET_RPC_URL must be set to an Ethereum mainnet RPC endpoint");
	let l2_url = std::env::var("ARB_MAINNET_RPC_URL")
		.expect("ARB_MAINNET_RPC_URL must be set to an Arbitrum One RPC endpoint");

	const EVENT_HEX: &str = "\
		02d02e25b72ad7ed901ab4b7720f4e98ea7ec6a723c844852cc1da91a420b20c\
		c020de75b5afca529758c5601f069edbe9d25c45c859a50d31c4c88cbdcb33f4\
		8a7513bf7bb3e3db04b0d982d0e973bcf57bf8b88aef7c6d03dba3a81a56a499\
		0000000000000000000000000000000000000000000000c328093e61ee400000\
		000000000000000000000000a5565d266c3c3ee90b16be8a5b13d587ef559fb0\
		000000000000000000000000000000000000000000000000000000000000b2fa\
		000000000000000000000000000000000000000000000000000000000012c506\
		deacbef6069ed87ba34122a8a8d8dbe9569f07d3e90cd0ef05dd12740f791f18\
		5e267bf69fffc988ee05ec3569b5dd7b52b2aeb3fcd1b92b98d33b188a030c6a\
		000000000000000000000000000000000000000000000000000000000012c4dc\
		0000000000000000000000000000000000000000000000000000000000000000\
		0000000000000000000000000000000000000000000000000000000000000001\
		16c4d20e53861eb214c769d89b53d813850a4f00393041f7b1a915c28a3b1b4a\
		2274cfd963c2cf0495a65e4f1d695e165c04a8ad58614a97c6b393c48791fe22\
		d74e7de82b2fbb23da849a14b41dd7f17e6ad4f0d991b2bf4f282d70bcaa3f88\
		000000000000000000000000000000000000000000000000000000000012c506\
		0000000000000000000000000000000000000000000000000000000000000000\
		0000000000000000000000000000000000000000000000000000000000000001\
		86e8c3d0402eb1f42ac3ebd92c349e467188f01428d5396e1a51c4cd140735e7\
		f4d4564c3958c4f2dc541d00a94a8a7c83fd759deaab96cbc6dbe21d311d7fae\
		000000000000000000000000000000000000000000000000000000000012c537\
		8a7513bf7bb3e3db04b0d982d0e973bcf57bf8b88aef7c6d03dba3a81a56a499\
		0000000000000000000000000000000000000000000000c328093e61ee400000\
		000000000000000000000000a5565d266c3c3ee90b16be8a5b13d587ef559fb0\
		000000000000000000000000000000000000000000000000000000000000b2fa";
	let data = hex::decode(EVENT_HEX).expect("event hex decodes");

	// The hex is the event's non-indexed ABI-encoded data; pass placeholder topics so
	// `decode_raw_log` populates the struct with indexed fields zeroed, then overwrite the
	// two indexed hashes below with the values derived from the payload.
	let topics: [alloy_primitives::B256; 3] = [
		IRollupBold::AssertionCreated::SIGNATURE_HASH,
		alloy_primitives::B256::ZERO,
		alloy_primitives::B256::ZERO,
	];
	let mut event = IRollupBold::AssertionCreated::decode_raw_log(topics, &data)
		.expect("AssertionCreated decodes");

	// Recompute the indexed hashes from the payload so they match what the rollup-core
	// actually indexed the assertion under — `fetch_arbitrum_bold_payload` queries
	// `_assertions[assertionHash]` directly and the verifier walks from parentAssertionHash.
	use alloy_sol_types::SolValue;
	let before_state_hash = keccak256(event.assertion.beforeState.abi_encode());
	let mut buf = Vec::with_capacity(96);
	buf.extend_from_slice(event.assertion.beforeStateData.prevPrevAssertionHash.as_slice());
	buf.extend_from_slice(before_state_hash.as_slice());
	buf.extend_from_slice(event.assertion.beforeStateData.sequencerBatchAcc.as_slice());
	let parent_hash = keccak256(&buf);

	let after_state_hash = keccak256(event.assertion.afterState.abi_encode());
	let mut buf = Vec::with_capacity(96);
	buf.extend_from_slice(parent_hash.as_slice());
	buf.extend_from_slice(after_state_hash.as_slice());
	buf.extend_from_slice(event.afterInboxBatchAcc.as_slice());
	let assertion_hash = keccak256(&buf);

	event.parentAssertionHash = parent_hash;
	event.assertionHash = assertion_hash;

	let host_config = HostConfig {
		ethereum_rpc_url: vec![l1_url],
		rollup_core: H160::from(hex!("4dceb440657f21083db8add07665f8ddbe1dcfc0")),
		// Ethereum mainnet chain id = 1.
		l1_state_machine: StateMachine::Evm(1),
		l1_consensus_state_id: "ETH0".to_string(),
		consensus_update_frequency: None,
	};
	let evm_config = EvmConfig {
		rpc_urls: vec![l2_url],
		consensus_state_id: "ETH0".to_string(),
		signer: DUMMY_SIGNING_KEY.to_string(),
		..Default::default()
	};
	let host = ArbHost::new(&host_config, &evm_config).await.expect("host");

	// Step back a handful of blocks so the L1 storage root we cite has settled past any
	// reorg — `fetch_arbitrum_bold_payload` pins its `get_proof` calls to this same block.
	let head = host.beacon_execution_client.get_block_number().await.expect("L1 head");
	let l1_block = head.saturating_sub(8);
	let l1_header = host
		.beacon_execution_client
		.get_block(BlockId::number(l1_block))
		.await
		.expect("L1 block")
		.expect("L1 block exists");
	let l1_state_root = H256::from_slice(l1_header.header.state_root.as_slice());

	let payload = host
		.fetch_arbitrum_bold_payload(l1_block, event)
		.await
		.expect("fetch_arbitrum_bold_payload");

	verify_arbitrum_bold::<Hasher>(payload, l1_state_root, host.rollup_core, Default::default())
		.expect("Arbitrum BoLD assertion proof must verify at latest L1 head");
}

#[test]
fn test_block_decoding() {
	let bytes = hex::decode("f90285f90219a0f052d217bd5275a5177a3c3b7debdfe2670f1c8394b2965ccd5c1883cc1a524da01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347948888f1f195afa192cfee860698584c030f4c9db1a0bac6177a79e910c98d86ec31a09ae37ac2de15b754fd7bed1ba52362c49416bfa0498785da562aa0c5dd5937cf15f22139b0b1bcf3b4fc48986e1bb1dae9292796a0c7778a7376099ee2e5c455791c1885b5c361b95713fddcbe32d97fd01334d296b90100000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000400000000000000000000000000000000000000000000000000000008302000001832fefba82560b845754130ea00102030405060708091011121314151617181920212223242526272829303132a0a4db124f7da8798b0467e96875d72bb656b0efb293af8bfc1f879468162973ae88edb3a329535a66a0f866f864800a82c35094095e7baea6a6c7c4c2dfeb977efac326af552d8785012a05f200801ca0ee0b9ec878fbd4258a9473199d8ecc32996a20c323c004e79e0cda20e0418ce3a04e6bc63927d1510bab54f37e46fa036faf4b2c465d271920d9afea1fadf7bd21c0").unwrap();
	let block: Block = Decodable::decode(&mut &*bytes).unwrap();

	let encoding = alloy_rlp::encode(block.clone()).to_vec();

	let header_encoding = alloy_rlp::encode(block.header.clone()).to_vec();
	let hash: H256 = keccak256(&header_encoding).0.into();

	assert_eq!(hash.0, hex!("590075f673be110bb0c0320ca89dea14877d5403dabbb54e45509449cbad0b14"));
	assert_eq!(hex::encode(bytes), hex::encode(encoding));
}
