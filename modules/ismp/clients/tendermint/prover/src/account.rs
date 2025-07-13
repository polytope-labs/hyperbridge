use cometbft::{account::Id as CometbftAccountId, public_key::PublicKey};

/// Custom account ID that matches Go CometBFT fork's address calculation
/// For Secp256k1: uses Keccak256 (Ethereum-style) instead of RIPEMD160(SHA256)
pub fn custom_account_id_from_pubkey(pub_key: &PublicKey) -> CometbftAccountId {
	match pub_key {
		PublicKey::Ed25519(pk) => {
			// SHA256(pk)[:20] - same as standard
			use sha2::{Digest, Sha256};
			let digest = Sha256::digest(pk.as_bytes());
			CometbftAccountId::new(digest[..20].try_into().unwrap())
		},
		PublicKey::Secp256k1(pk) => {
			// Keccak256(pubkey)[12:] - Ethereum-style like Go fork
			use sha3::{Digest, Keccak256};
			let pubkey_bytes = pk.to_encoded_point(false).as_bytes().to_vec();
			// Remove the 0x04 prefix (first byte) as done in Go implementation
			let keccak_hash = Keccak256::digest(&pubkey_bytes[1..]);
			// Take last 20 bytes (bytes 12-31)
			CometbftAccountId::new(keccak_hash[12..32].try_into().unwrap())
		},
		#[allow(unreachable_patterns)]
		_ => {
			// Catch-all for non_exhaustive enum
			panic!("Unsupported public key type for Polygon/Heimdall verification")
		},
	}
}
