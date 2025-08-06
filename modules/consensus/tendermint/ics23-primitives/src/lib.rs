#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_sdk::sp_io;

pub struct ICS23HostFunctions;

impl ics23::HostFunctionsProvider for ICS23HostFunctions {
	fn sha2_256(message: &[u8]) -> [u8; 32] {
		sp_io::hashing::sha2_256(message)
	}

	fn sha2_512(message: &[u8]) -> [u8; 64] {
		use sha2::{Digest, Sha512};
		let mut hasher = Sha512::new();
		hasher.update(message);
		hasher.finalize().into()
	}

	fn sha2_512_truncated(message: &[u8]) -> [u8; 32] {
		use sha2::{Digest, Sha512_256};
		let mut hasher = Sha512_256::new();
		hasher.update(message);
		hasher.finalize().into()
	}

	fn keccak_256(message: &[u8]) -> [u8; 32] {
		sp_io::hashing::keccak_256(message)
	}

	fn ripemd160(message: &[u8]) -> [u8; 20] {
		use ripemd::{Digest, Ripemd160};
		let mut hasher = Ripemd160::new();
		hasher.update(message);
		hasher.finalize().into()
	}

	fn blake2b_512(message: &[u8]) -> [u8; 64] {
		use blake2::{Blake2b, Digest};
		let mut hasher = Blake2b::new();
		hasher.update(message);
		hasher.finalize().into()
	}

	fn blake2s_256(message: &[u8]) -> [u8; 32] {
		use blake2::{Blake2s, Digest};
		let mut hasher = Blake2s::new();
		hasher.update(message);
		hasher.finalize().into()
	}

	fn blake3(message: &[u8]) -> [u8; 32] {
		blake3::hash(message).into()
	}
}
