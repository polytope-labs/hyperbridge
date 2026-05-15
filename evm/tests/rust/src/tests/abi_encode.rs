use super::utils::*;
use alloy_primitives::Bytes;
use alloy_sol_types::SolValue;
use ismp::{
	host::StateMachine,
	messaging::{hash_get_response, hash_request},
	router::{GetRequest, GetResponse, PostRequest, Request, StorageValue},
};
use ismp_abi::evm_host::EvmHost;

fn deploy_codec(env: &mut TestEnv) -> alloy_primitives::Address {
	let out_dir = env.evm_out_dir_public();
	env.deploy_named(&out_dir, "AbiCodec")
}

/// Build calldata: 4-byte selector + abi-encoded params
fn calldata(selector: [u8; 4], params: &[u8]) -> Vec<u8> {
	[&selector[..], params].concat()
}

// Selectors from AbiCodec contract (forge inspect AbiCodec methodIdentifiers)
const ENCODE_POST: [u8; 4] = [0x75, 0x09, 0xf2, 0x88];
const DECODE_POST: [u8; 4] = [0x55, 0x75, 0x2f, 0x4c];
const HASH_POST: [u8; 4] = [0xf8, 0xd4, 0x4b, 0x7b];
const ENCODE_GET: [u8; 4] = [0xa3, 0x57, 0x49, 0xe4];
const DECODE_GET: [u8; 4] = [0x82, 0x80, 0xca, 0xca];
const HASH_GET: [u8; 4] = [0x6e, 0xd6, 0xee, 0x5c];
const ENCODE_GET_RESP: [u8; 4] = [0xcc, 0x17, 0x19, 0x80];
const DECODE_GET_RESP: [u8; 4] = [0x9d, 0x66, 0xfe, 0xc1];
const HASH_GET_RESP: [u8; 4] = [0xa3, 0x5d, 0xd3, 0x1a];

fn sample_post_request() -> PostRequest {
	PostRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 42,
		from: hex::decode("deadbeef").unwrap(),
		to: hex::decode("cafebabe").unwrap(),
		timeout_timestamp: 1000,
		body: hex::decode("1234").unwrap(),
	}
}

fn sample_get_request() -> GetRequest {
	GetRequest {
		source: StateMachine::Polkadot(2000),
		dest: StateMachine::Evm(1),
		nonce: 7,
		from: hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap(),
		keys: vec![hex::decode("aabb").unwrap(), hex::decode("ccdd").unwrap()],
		height: 100,
		context: hex::decode("ff").unwrap(),
		timeout_timestamp: 500,
	}
}

fn sample_get_response() -> GetResponse {
	GetResponse {
		get: GetRequest {
			source: StateMachine::Polkadot(2000),
			dest: StateMachine::Evm(1),
			nonce: 1,
			from: hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap(),
			keys: vec![hex::decode("aabb").unwrap()],
			height: 50,
			context: vec![],
			timeout_timestamp: 500,
		},
		values: vec![StorageValue {
			key: hex::decode("aabb").unwrap(),
			value: Some(hex::decode("1122").unwrap()),
		}],
	}
}

#[test]
fn test_post_request_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let req = sample_post_request();
	let sol_req: EvmHost::PostRequest = req.clone().into();

	// Rust encodes
	let rust_encoded = Request::Post(req.clone()).encode();

	// Solidity encodes
	let result = env.call(codec, calldata(ENCODE_POST, &sol_req.abi_encode()));
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	// Encoding must be identical
	assert_eq!(rust_encoded, sol_encoded, "PostRequest encoding mismatch");

	// Solidity can decode Rust-encoded bytes
	let result = env.call(codec, calldata(DECODE_POST, &Bytes::from(rust_encoded.clone()).abi_encode()));
	let decoded = EvmHost::PostRequest::abi_decode(&result).unwrap();
	assert_eq!(decoded.nonce, 42);
	assert_eq!(decoded.source.to_vec(), b"POLKADOT-2000");

	// Hashes match
	let rust_hash = hash_request::<crate::Keccak256>(&Request::Post(req));
	let result = env.call(codec, calldata(HASH_POST, &sol_req.abi_encode()));
	let sol_hash = <alloy_primitives::FixedBytes<32>>::abi_decode(&result).unwrap();
	assert_eq!(rust_hash.0, sol_hash.0, "PostRequest hash mismatch");
}

#[test]
fn test_get_request_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let req = sample_get_request();
	let sol_req: EvmHost::GetRequest = req.clone().into();

	let rust_encoded = Request::Get(req.clone()).encode();

	let result = env.call(codec, calldata(ENCODE_GET, &sol_req.abi_encode()));
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	assert_eq!(rust_encoded, sol_encoded, "GetRequest encoding mismatch");

	// Solidity decodes Rust bytes
	env.call(codec, calldata(DECODE_GET, &Bytes::from(rust_encoded).abi_encode()));

	// Hashes match
	let rust_hash = hash_request::<crate::Keccak256>(&Request::Get(req));
	let result = env.call(codec, calldata(HASH_GET, &sol_req.abi_encode()));
	let sol_hash = <alloy_primitives::FixedBytes<32>>::abi_decode(&result).unwrap();
	assert_eq!(rust_hash.0, sol_hash.0, "GetRequest hash mismatch");
}

#[test]
fn test_get_response_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let res = sample_get_response();
	let sol_res: EvmHost::GetResponse = res.clone().into();

	let rust_encoded = res.encode();

	let result = env.call(codec, calldata(ENCODE_GET_RESP, &sol_res.abi_encode()));
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	assert_eq!(rust_encoded, sol_encoded, "GetResponse encoding mismatch");

	// Solidity decodes Rust bytes
	env.call(codec, calldata(DECODE_GET_RESP, &Bytes::from(rust_encoded).abi_encode()));

	// Hashes match
	let rust_hash = hash_get_response::<crate::Keccak256>(&res);
	let result = env.call(codec, calldata(HASH_GET_RESP, &sol_res.abi_encode()));
	let sol_hash = <alloy_primitives::FixedBytes<32>>::abi_decode(&result).unwrap();
	assert_eq!(rust_hash.0, sol_hash.0, "GetResponse hash mismatch");
}
