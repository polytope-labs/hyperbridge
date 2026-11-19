//! Request commitments as the EVM host computes them: `keccak256(abi.encode(request))`, with the
//! struct layouts of `Message.sol`. `abi_encode` keeps the outer tuple wrapper, as
//! `abi.encode(struct)` does; `abi_encode_params` would not.

use alloy::{
	primitives::{keccak256, Bytes, B256},
	sol,
	sol_types::SolValue,
};

sol! {
	struct PostRequest {
		bytes source;
		bytes dest;
		uint64 nonce;
		bytes from;
		bytes to;
		uint64 timeoutTimestamp;
		bytes body;
	}

	struct GetRequest {
		bytes source;
		bytes dest;
		uint64 nonce;
		bytes from;
		uint64 timeoutTimestamp;
		bytes[] keys;
		uint64 height;
		bytes context;
	}
}

/** `RequestService.computeRequestCommitment`. `source` and `dest` are state machine strings. */
pub fn post_request_commitment(
	source: &str,
	dest: &str,
	nonce: u64,
	timeout_timestamp: u64,
	from: &[u8],
	to: &[u8],
	body: &[u8],
) -> B256 {
	let request = PostRequest {
		source: Bytes::copy_from_slice(source.as_bytes()),
		dest: Bytes::copy_from_slice(dest.as_bytes()),
		nonce,
		from: Bytes::copy_from_slice(from),
		to: Bytes::copy_from_slice(to),
		timeoutTimestamp: timeout_timestamp,
		body: Bytes::copy_from_slice(body),
	};
	keccak256(request.abi_encode())
}

/** `GetRequestService.computeRequestCommitment`. */
#[allow(clippy::too_many_arguments)]
pub fn get_request_commitment(
	source: &str,
	dest: &str,
	nonce: u64,
	height: u64,
	timeout_timestamp: u64,
	from: &[u8],
	keys: &[Bytes],
	context: &[u8],
) -> B256 {
	let request = GetRequest {
		source: Bytes::copy_from_slice(source.as_bytes()),
		dest: Bytes::copy_from_slice(dest.as_bytes()),
		nonce,
		from: Bytes::copy_from_slice(from),
		timeoutTimestamp: timeout_timestamp,
		keys: keys.to_vec(),
		height,
		context: Bytes::copy_from_slice(context),
	};
	keccak256(request.abi_encode())
}

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::{
		host::StateMachine,
		router::{GetRequest as IsmpGet, PostRequest as IsmpPost},
	};

	fn hex(value: &str) -> Vec<u8> {
		hex::decode(value.trim_start_matches("0x")).unwrap()
	}

	/** `getRequest.service.test.ts`: a Sepolia → Base Sepolia GET from issue #1013. */
	#[test]
	fn get_commitment_matches_the_host() {
		let keys = [Bytes::from(hex(
			"0x397cfd836f98eed9fad1041a07de20fe6abaa389ac33ff75c19e70fe83507db0d683fd3465c996598dc972688b7ace676c89077b",
		))];
		let commitment = get_request_commitment(
			"EVM-11155111",
			"EVM-84532",
			2,
			0,
			0,
			&hex("0x7d6b92fa404123e6398d2de862d967f95533c90d"),
			&keys,
			&[0u8; 32],
		);
		assert_eq!(
			format!("{commitment:#x}"),
			"0xdaa89209aac40e56d0d53dd5105aef394da36a149dc3e226c10b08897303a379"
		);
	}

	#[test]
	fn commitments_match_the_ismp_crate() {
		let from = hex("0xae041f7b0cb581876832830baeb6a2aa2a3c9716");
		let post = IsmpPost {
			source: StateMachine::Evm(56),
			dest: StateMachine::Polkadot(3367),
			nonce: 7,
			from: from.clone(),
			to: hex("0x1111111111111111111111111111111111111111"),
			timeout_timestamp: 1_700_000_000,
			body: hex("0x1234"),
		};
		let ours = post_request_commitment(
			"EVM-56",
			"POLKADOT-3367",
			7,
			1_700_000_000,
			&post.from,
			&post.to,
			&post.body,
		);
		assert_eq!(ours, keccak256(ismp::abi::encode_post_request(&post)));

		let get = IsmpGet {
			source: StateMachine::Evm(1),
			dest: StateMachine::Kusama(4009),
			nonce: 3,
			from,
			keys: vec![hex("0x01"), hex("0x0203")],
			height: 99,
			context: hex("0xbeef"),
			timeout_timestamp: 5,
		};
		let keys: Vec<Bytes> = get.keys.iter().cloned().map(Bytes::from).collect();
		let ours = get_request_commitment(
			"EVM-1",
			"KUSAMA-4009",
			3,
			99,
			5,
			&get.from,
			&keys,
			&get.context,
		);
		assert_eq!(ours, keccak256(ismp::abi::encode_get_request(&get)));
	}
}
