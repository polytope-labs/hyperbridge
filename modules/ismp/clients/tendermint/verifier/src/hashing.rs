use polkadot_sdk::sp_io;
use tendermint::{
	crypto::{signature, Sha256},
	merkle::MerkleHash,
	PublicKey, Signature,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoSha256;

impl Sha256 for SpIoSha256 {
	fn digest(data: impl AsRef<[u8]>) -> [u8; 32] {
		sp_io::hashing::sha2_256(data.as_ref())
	}
}

impl MerkleHash for SpIoSha256 {
	fn empty_hash(&mut self) -> [u8; 32] {
		Self::digest(&[])
	}

	fn leaf_hash(&mut self, bytes: &[u8]) -> [u8; 32] {
		Self::digest(bytes)
	}

	fn inner_hash(&mut self, left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
		let mut combined = Vec::new();
		combined.extend_from_slice(&left);
		combined.extend_from_slice(&right);
		Self::digest(combined)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoSignatureVerifier;

impl signature::Verifier for SpIoSignatureVerifier {
	fn verify(
		pubkey: PublicKey,
		msg: &[u8],
		signature: &Signature,
	) -> Result<(), signature::Error> {
		match pubkey {
			PublicKey::Ed25519(pk) => {
				let sig = polkadot_sdk::sp_core::ed25519::Signature::try_from(signature.as_bytes())
					.map_err(|_| signature::Error::MalformedSignature)?;
				let pub_key = polkadot_sdk::sp_core::ed25519::Public::try_from(pk.as_bytes())
					.map_err(|_| signature::Error::MalformedPublicKey)?;

				if sp_io::crypto::ed25519_verify(&sig, msg, &pub_key) {
					Ok(())
				} else {
					Err(signature::Error::VerificationFailed)
				}
			},
			#[cfg(feature = "secp256k1")]
			PublicKey::Secp256k1(pk) => {
				let sig = polkadot_sdk::sp_core::ecdsa::Signature::try_from(signature.as_bytes())
					.map_err(|_| signature::Error::MalformedSignature)?;
				let pub_key = polkadot_sdk::sp_core::ecdsa::Public::try_from(pk.as_bytes())
					.map_err(|_| signature::Error::MalformedPublicKey)?;

				if sp_io::crypto::ecdsa_verify(&sig, msg, &pub_key) {
					Ok(())
				} else {
					Err(signature::Error::VerificationFailed)
				}
			},
			_ => Err(signature::Error::UnsupportedKeyType),
		}
	}
}
