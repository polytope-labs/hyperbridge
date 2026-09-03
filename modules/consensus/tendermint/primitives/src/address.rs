//! Validator account id derivation.

use cometbft::{account, public_key::PublicKey};
use sha2::{Digest, Sha256};
use sha3::Keccak256;

/// Length of a Tendermint account id in bytes.
const ACCOUNT_ID_LEN: usize = 20;

/// Recomputes the account id that a validator must present for a given public key.
///
/// Ed25519 keys use the usual sha256 truncation shared with vanilla CometBFT.
/// Secp256k1 keys follow the Ethereum style derivation Heimdall adopted: keccak
/// over the uncompressed key with its leading tag byte dropped, taking the last
/// twenty bytes. Returns `None` for key types we do not recognise.
pub fn account_id_from_public_key(pub_key: &PublicKey) -> Option<account::Id> {
	match pub_key {
		PublicKey::Ed25519(key) => {
			let digest = Sha256::digest(key.as_bytes());
			Some(account::Id::new(digest[..ACCOUNT_ID_LEN].try_into().ok()?))
		},
		PublicKey::Secp256k1(key) => {
			let encoded = key.to_encoded_point(false);
			let digest = Keccak256::digest(&encoded.as_bytes()[1..]);
			Some(account::Id::new(digest[12..32].try_into().ok()?))
		},
		_ => None,
	}
}
