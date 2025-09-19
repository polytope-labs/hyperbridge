use alloy_primitives::{Address, Bytes, U256};
use alloy_rlp::Decodable;
// use arbitrum_verifier::verify_arbitrum_payload;
use geth_primitives::Header;
// use ismp_testsuite::mocks::Host;
use tesseract_evm::EvmConfig;

use crate::{ArbConfig, ArbHost, HostConfig};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use ethers::{providers::Middleware, utils::keccak256};
use hex_literal::hex;
use ismp::host::StateMachine;
use primitive_types::{H160, H256};

const ROLLUP_CORE: [u8; 20] = hex!("d80810638dbDF9081b72C1B33c65375e807281C8");

#[tokio::test]
#[ignore]
async fn test_payload_proof_verification() {
	dotenv::dotenv().ok();
	let arb_url = std::env::var("ARB_URL").expect("ARB_URL must be set.");
	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
	let host = HostConfig {
		ethereum_rpc_url: vec![geth_url],
		rollup_core: H160::from_slice(&ROLLUP_CORE),
		l1_state_machine: StateMachine::Evm(10),
		l1_consensus_state_id: "ETH0".to_string(),
		consensus_update_frequency: None,
	};
	let config = ArbConfig {
		host: host.clone(),
		evm_config: EvmConfig {
			rpc_urls: vec![arb_url],
			consensus_state_id: "ETH0".to_string(),
			..Default::default()
		},
	};

	let arb_client = ArbHost::new(&host, &config.evm_config).await.expect("Host creation failed");

	let event = arb_client
		.latest_event(5524107, 5524107)
		.await
		.expect("Failed to fetch latest event")
		.expect("There should be an event");

	let _payload_proof = arb_client
		.fetch_arbitrum_payload(5524107, event)
		.await
		.expect("Error fetching payload proof");

	let l1_header = arb_client
		.beacon_execution_client
		.get_block(5524107)
		.await
		.unwrap()
		.expect("Block should exist");

	let _state_root = l1_header.state_root;

	// let _ = verify_arbitrum_payload::<Host>(
	// 	payload_proof,
	// 	state_root,
	// 	arb_client.rollup_core,
	// 	Default::default(),
	// )
	// .expect("Payload proof verification should succeed");
}

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

#[test]
fn test_block_decoding() {
	let bytes = hex::decode("f90285f90219a0f052d217bd5275a5177a3c3b7debdfe2670f1c8394b2965ccd5c1883cc1a524da01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347948888f1f195afa192cfee860698584c030f4c9db1a0bac6177a79e910c98d86ec31a09ae37ac2de15b754fd7bed1ba52362c49416bfa0498785da562aa0c5dd5937cf15f22139b0b1bcf3b4fc48986e1bb1dae9292796a0c7778a7376099ee2e5c455791c1885b5c361b95713fddcbe32d97fd01334d296b90100000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000400000000000000000000000000000000000000000000000000000008302000001832fefba82560b845754130ea00102030405060708091011121314151617181920212223242526272829303132a0a4db124f7da8798b0467e96875d72bb656b0efb293af8bfc1f879468162973ae88edb3a329535a66a0f866f864800a82c35094095e7baea6a6c7c4c2dfeb977efac326af552d8785012a05f200801ca0ee0b9ec878fbd4258a9473199d8ecc32996a20c323c004e79e0cda20e0418ce3a04e6bc63927d1510bab54f37e46fa036faf4b2c465d271920d9afea1fadf7bd21c0").unwrap();
	let block: Block = Decodable::decode(&mut &*bytes).unwrap();

	let encoding = alloy_rlp::encode(block.clone()).to_vec();

	let header_encoding = alloy_rlp::encode(block.header.clone()).to_vec();
	let hash: H256 = keccak256(&header_encoding).into();

	assert_eq!(hash.0, hex!("590075f673be110bb0c0320ca89dea14877d5403dabbb54e45509449cbad0b14"));
	assert_eq!(hex::encode(bytes), hex::encode(encoding));
}
