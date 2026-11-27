use rand::RngCore;
use sp_core::{
	crypto::{AccountId32, Ss58AddressFormat, Ss58Codec},
	ecdsa, ed25519, sr25519, Pair,
};
use sp_crypto_hashing::{blake2_256, sha2_256};

/// Nexus's SS58 prefix, which omniplex writes every address in.
const SS58_PREFIX: u16 = 0;

/// An account in the chain's address format.
pub fn ss58(account: AccountId32) -> String {
	account.to_ss58check_with_version(Ss58AddressFormat::custom(SS58_PREFIX))
}

/// The address in the chain's format, whatever SS58 prefix it was given in, so one key is always
/// one account. `None` when it is not an address.
pub fn canonical(address: &str) -> Option<String> {
	AccountId32::from_ss58check(address.trim()).ok().map(ss58)
}

/// What the database keeps of a session token, so reading the table does not hand out sessions.
pub fn token_hash(token: &str) -> String {
	hex::encode(sha2_256(token.as_bytes()))
}

/// Random bytes as hex, for login challenges and session tokens.
pub fn random_hex(bytes: usize) -> String {
	let mut buffer = vec![0u8; bytes];
	rand::thread_rng().fill_bytes(&mut buffer);
	hex::encode(buffer)
}

/// Checks that `signature` was made by `address` over `message`. Browser wallets wrap the message
/// in `<Bytes>` tags before signing, and some prefix the signature with a byte naming its scheme,
/// so every form is accepted.
pub fn verify(address: &str, message: &str, signature: &str) -> bool {
	let Ok(account) = AccountId32::from_ss58check(address) else { return false };
	let Ok(signature) = hex::decode(signature.trim_start_matches("0x")) else { return false };
	let account = <[u8; 32]>::from(account);
	let wrapped = format!("<Bytes>{message}</Bytes>");

	let signed = [wrapped.as_bytes(), message.as_bytes()]
		.into_iter()
		.any(|message| signed_by(&account, message, &signature));
	signed
}

fn signed_by(account: &[u8; 32], message: &[u8], signature: &[u8]) -> bool {
	let sr25519 = |signature: &[u8]| {
		sr25519::Signature::try_from(signature).is_ok_and(|sig| {
			sr25519::Pair::verify(&sig, message, &sr25519::Public::from_raw(*account))
		})
	};
	let ed25519 = |signature: &[u8]| {
		ed25519::Signature::try_from(signature).is_ok_and(|sig| {
			ed25519::Pair::verify(&sig, message, &ed25519::Public::from_raw(*account))
		})
	};
	// An ecdsa account id is the blake2 hash of its compressed public key
	let ecdsa = |signature: &[u8]| {
		ecdsa::Signature::try_from(signature)
			.ok()
			.and_then(|sig| sig.recover(message))
			.is_some_and(|public| blake2_256(public.as_ref()) == *account)
	};

	match signature {
		// An unprefixed 65 byte ecdsa signature can start with either prefix byte too
		[0, rest @ ..] if rest.len() == 64 => ed25519(rest) || ecdsa(signature),
		[1, rest @ ..] if rest.len() == 64 => sr25519(rest) || ecdsa(signature),
		[2, rest @ ..] if rest.len() == 65 => ecdsa(rest),
		_ if signature.len() == 64 => sr25519(signature) || ed25519(signature),
		_ => ecdsa(signature),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const CHALLENGE: &str = "challenge";

	fn address(account: [u8; 32]) -> String {
		ss58(AccountId32::from(account))
	}

	fn wrapped() -> String {
		format!("<Bytes>{CHALLENGE}</Bytes>")
	}

	#[test]
	fn accepts_wallet_signatures_of_every_scheme() {
		let sr = sr25519::Pair::from_string("//Alice", None).unwrap();
		let ed = ed25519::Pair::from_string("//Bob", None).unwrap();
		let ec = ecdsa::Pair::from_string("//Charlie", None).unwrap();

		let sr_signature = sr.sign(wrapped().as_bytes());
		assert!(verify(&address(sr.public().0), CHALLENGE, &hex::encode(sr_signature.0)));
		assert!(verify(
			&address(sr.public().0),
			CHALLENGE,
			&hex::encode([&[1u8][..], &sr_signature.0].concat())
		));

		let ed_signature = ed.sign(CHALLENGE.as_bytes());
		assert!(verify(
			&address(ed.public().0),
			CHALLENGE,
			&format!("0x{}", hex::encode(ed_signature.0))
		));

		let ec_signature = ec.sign(wrapped().as_bytes());
		let ec_address = address(blake2_256(ec.public().as_ref()));
		assert!(verify(
			&ec_address,
			CHALLENGE,
			&hex::encode([&[2u8][..], &ec_signature.0].concat())
		));
	}

	#[test]
	fn accepts_unprefixed_ecdsa_signatures_that_look_prefixed() {
		let ec = ecdsa::Pair::from_string("//Charlie", None).unwrap();
		let ec_address = address(blake2_256(ec.public().as_ref()));
		for first in [0u8, 1] {
			// Signing is deterministic, so search the messages for a signature with that first byte
			let (message, signature) = (0..10_000)
				.map(|nonce| format!("{CHALLENGE} {nonce}"))
				.map(|message| {
					let signature = ec.sign(message.as_bytes()).0;
					(message, signature)
				})
				.find(|(_, signature)| signature[0] == first)
				.unwrap();
			assert!(verify(&ec_address, &message, &hex::encode(signature)));
		}
	}

	#[test]
	fn gives_one_address_for_a_key_whatever_its_prefix() {
		let alice = sr25519::Pair::from_string("//Alice", None).unwrap();
		let account = AccountId32::from(alice.public().0);
		let polkadot = address(alice.public().0);
		let generic = account.to_ss58check_with_version(Ss58AddressFormat::custom(42));
		let kusama = account.to_ss58check_with_version(Ss58AddressFormat::custom(2));
		assert_ne!(polkadot, generic);
		assert_eq!(canonical(&generic).as_deref(), Some(polkadot.as_str()));
		assert_eq!(canonical(&kusama).as_deref(), Some(polkadot.as_str()));
		assert_eq!(canonical(&polkadot).as_deref(), Some(polkadot.as_str()));
		assert_eq!(canonical("not an address"), None);
	}

	#[test]
	fn rejects_signatures_from_another_account_or_message() {
		let alice = sr25519::Pair::from_string("//Alice", None).unwrap();
		let bob = sr25519::Pair::from_string("//Bob", None).unwrap();
		let signature = hex::encode(alice.sign(wrapped().as_bytes()).0);

		assert!(!verify(&address(bob.public().0), CHALLENGE, &signature));
		assert!(!verify(&address(alice.public().0), "another challenge", &signature));
		assert!(!verify(&address(alice.public().0), CHALLENGE, "not hex"));
		assert!(!verify("not an address", CHALLENGE, &signature));
	}
}
