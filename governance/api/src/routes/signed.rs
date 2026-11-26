use chrono::Utc;
use codec::Encode;
use serde::Deserialize;
use serde_json::{value::RawValue, Value};
use sp_core::crypto::{AccountId32, Ss58Codec};
use sp_crypto_hashing::blake2_256;

use super::AppState;
use crate::{
	crypto::{ss58, verify},
	error::{ApiError, ApiResult},
};

/// How long a signed request stays valid, so an old signature cannot be replayed later.
const VALID_FOR_MS: i64 = 10 * 60 * 1000;
/// How far ahead of the server's clock a signer's clock may be.
const MAX_SKEW_MS: i64 = 60 * 1000;

/// A change an account signed with its wallet instead of a session: the frontend signs
/// `JSON.stringify(entity)`, so the entity is kept exactly as sent to check the signature against.
/// The entity names the forum's domain, so a request signed for another forum built from the same
/// frontend is refused here.
#[derive(Deserialize)]
pub struct SignedEntity {
	entity: Box<RawValue>,
	address: String,
	signature: String,
}

/// A signed request that checked out, with the signer in the chain's address format.
pub struct Signed {
	pub entity: Value,
	pub address: String,
}

impl SignedEntity {
	/// The entity and its signer, once its signature, action and age check out. Each signed request
	/// is acted on once, so one seen before is turned away.
	pub async fn verify(&self, state: &AppState, action: &str) -> ApiResult<Signed> {
		let (signed, account, timestamp) = self.check(action, &state.domain)?;
		let fresh = state
			.db
			.get()
			.await?
			.execute(
				"INSERT INTO used_signatures (hash, expires_at)
				VALUES ($1, to_timestamp($2::double precision / 1000)) ON CONFLICT DO NOTHING",
				&[
					&blake2_256(&(account, self.entity.get()).encode()).to_vec(),
					&((timestamp + VALID_FOR_MS) as f64),
				],
			)
			.await?;
		if fresh == 0 {
			return Err(ApiError::bad_request("This signed request was already used"));
		}
		Ok(signed)
	}

	/// The checks that need no database, returning the request with the signing account and the
	/// signed time.
	fn check(&self, action: &str, domain: &str) -> ApiResult<(Signed, [u8; 32], i64)> {
		let account = AccountId32::from_ss58check(self.address.trim())
			.map_err(|_| ApiError::bad_request("Invalid address"))?;
		if !verify(&self.address, self.entity.get(), &self.signature) {
			return Err(ApiError::bad_request("Invalid signature"));
		}
		let entity = serde_json::from_str::<Value>(self.entity.get())
			.map_err(|_| ApiError::bad_request("Invalid request"))?;
		if entity["action"] != action {
			return Err(ApiError::bad_request("The signed action does not match this request"));
		}
		if entity["domain"] != domain {
			return Err(ApiError::bad_request("The request was signed for another site"));
		}
		// Signing as a proxy would need the proxy checked on chain, which is not supported yet
		if !entity["real"].is_null() {
			return Err(ApiError::bad_request("Signing for a proxied account is not supported yet"));
		}
		let timestamp = entity["timestamp"].as_i64().unwrap_or_default();
		if !(-MAX_SKEW_MS..VALID_FOR_MS).contains(&(Utc::now().timestamp_millis() - timestamp)) {
			return Err(ApiError::bad_request(
				"The signed request has expired, please sign it again",
			));
		}
		let raw = <[u8; 32]>::from(account.clone());
		Ok((Signed { entity, address: ss58(account) }, raw, timestamp))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;
	use sp_core::{
		crypto::{AccountId32, Ss58AddressFormat, Ss58Codec},
		sr25519, Pair,
	};

	const DOMAIN: &str = "forum.example";

	fn request(
		signer: &sr25519::Pair,
		address: &sr25519::Pair,
		entity: Value,
	) -> anyhow::Result<SignedEntity> {
		let signature = format!(
			"0x{}",
			hex::encode(signer.sign(format!("<Bytes>{entity}</Bytes>").as_bytes()).0)
		);
		// Signed in the generic format, which the server turns into the chain's
		let address = AccountId32::from(address.public().0)
			.to_ss58check_with_version(Ss58AddressFormat::custom(42));
		Ok(serde_json::from_value(
			json!({ "entity": entity, "address": address, "signature": signature }),
		)?)
	}

	#[test]
	fn accepts_only_fresh_matching_signatures_from_the_account() -> anyhow::Result<()> {
		let (alice, bob) = (
			sr25519::Pair::from_string("//Alice", None)?,
			sr25519::Pair::from_string("//Bob", None)?,
		);
		let now = Utc::now().timestamp_millis();
		let bio = |timestamp: i64| json!({ "action": "set-profile-bio", "bio": "hi", "timestamp": timestamp, "domain": DOMAIN });

		let (signed, ..) = request(&alice, &alice, bio(now))?
			.check("set-profile-bio", DOMAIN)
			.map_err(|_| anyhow::anyhow!("a fresh request is accepted"))?;
		assert_eq!(signed.address, ss58(AccountId32::from(alice.public().0)));
		assert!(
			request(&alice, &alice, bio(now))?.check("set-avatar", DOMAIN).is_err(),
			"another action"
		);
		assert!(
			request(&bob, &alice, bio(now))?.check("set-profile-bio", DOMAIN).is_err(),
			"another signer"
		);
		assert!(
			request(&alice, &alice, bio(now - VALID_FOR_MS - 1))?
				.check("set-profile-bio", DOMAIN)
				.is_err(),
			"too old"
		);
		assert!(
			request(&alice, &alice, bio(now + 2 * MAX_SKEW_MS))?
				.check("set-profile-bio", DOMAIN)
				.is_err(),
			"too far ahead"
		);
		assert!(
			request(&alice, &alice, bio(now + MAX_SKEW_MS / 2))?
				.check("set-profile-bio", DOMAIN)
				.is_ok(),
			"a little clock skew is fine"
		);
		let elsewhere =
			json!({ "action": "set-profile-bio", "timestamp": now, "domain": "other.example" });
		assert!(
			request(&alice, &alice, elsewhere)?.check("set-profile-bio", DOMAIN).is_err(),
			"signed for another forum"
		);
		let unnamed = json!({ "action": "set-profile-bio", "timestamp": now });
		assert!(
			request(&alice, &alice, unnamed)?.check("set-profile-bio", DOMAIN).is_err(),
			"names no forum"
		);
		let proxied = json!({
			"action": "set-profile-bio", "timestamp": now, "real": "someone", "domain": DOMAIN
		});
		assert!(
			request(&alice, &alice, proxied)?.check("set-profile-bio", DOMAIN).is_err(),
			"signed as a proxy"
		);
		Ok(())
	}
}
