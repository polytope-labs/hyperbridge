use rand::RngCore;
use sp_core::{
	crypto::{AccountId32, Ss58Codec},
	ecdsa, ed25519, sr25519, Pair,
};
use sp_crypto_hashing::blake2_256;

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
		[0, rest @ ..] if rest.len() == 64 => ed25519(rest),
		[1, rest @ ..] if rest.len() == 64 => sr25519(rest),
		[2, rest @ ..] if rest.len() == 65 => ecdsa(rest),
		_ if signature.len() == 64 => sr25519(signature) || ed25519(signature),
		_ => ecdsa(signature),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_core::crypto::Ss58AddressFormat;

	const CHALLENGE: &str = "challenge";

	fn address(account: [u8; 32]) -> String {
		AccountId32::from(account).to_ss58check_with_version(Ss58AddressFormat::custom(0))
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
