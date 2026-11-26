use std::collections::BTreeSet;

use axum::{
	extract::{Path, State},
	Json,
};
use codec::Encode;
use serde_json::{json, Value};
use sp_core::crypto::{AccountId32, Ss58Codec};
use sp_crypto_hashing::blake2_256;

use super::{common::valid_address, signed::SignedEntity, AppState};
use crate::{
	crypto::ss58,
	error::{ApiError, ApiResult},
};

/// The multisig account a set of signatories and a threshold control, derived as pallet-multisig
/// does, so a saved multisig always matches the account it signs for.
pub fn multisig_address(signatories: &[String], threshold: u16) -> Option<String> {
	let mut accounts = signatories
		.iter()
		.map(|signatory| AccountId32::from_ss58check(signatory).ok().map(<[u8; 32]>::from))
		.collect::<Option<Vec<_>>>()?;
	accounts.sort();
	accounts.dedup();
	let entropy = blake2_256(&(b"modlpy/utilisuba", accounts, threshold).encode());
	Some(ss58(AccountId32::from(entropy)))
}

/// The multisig accounts an account saved.
pub async fn list(
	State(state): State<AppState>,
	Path(owner): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
	let owner = valid_address(&owner)?;
	let rows = state
		.db
		.get()
		.await?
		.query(
			"SELECT multisig_address, name, threshold, signatories FROM multisigs WHERE owner = $1 ORDER BY name",
			&[&owner],
		)
		.await?;
	Ok(Json(
		rows.iter()
			.map(|row| {
				json!({
					"multisigAddress": row.get::<_, String>(0),
					"name": row.get::<_, String>(1),
					"threshold": row.get::<_, i16>(2),
					"signatories": row.get::<_, Value>(3),
				})
			})
			.collect(),
	))
}

/// The signed request, checked to come from the account whose list the route names, and that
/// account in the chain's format.
async fn owned_entity(
	state: &AppState,
	signed: &SignedEntity,
	owner: &str,
	action: &str,
) -> ApiResult<(Value, String)> {
	let owner = valid_address(owner)?;
	let signed = signed.verify(state, action).await?;
	if signed.address != owner {
		return Err(ApiError::forbidden());
	}
	Ok((signed.entity, owner))
}

fn name(entity: &Value) -> ApiResult<String> {
	let name = entity["name"].as_str().unwrap_or_default().trim();
	if name.is_empty() || name.chars().count() > 100 {
		return Err(ApiError::bad_request("The name must be between 1 and 100 characters"));
	}
	Ok(name.to_owned())
}

pub async fn add(
	State(state): State<AppState>,
	Path(owner): Path<String>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	let (entity, owner) = owned_entity(&state, &signed, &owner, "add-multisig").await?;
	let given = serde_json::from_value::<Vec<String>>(entity["signatories"].clone())
		.map_err(|_| ApiError::bad_request("Signatories must be a list of addresses"))?;
	let signatories = given
		.iter()
		.map(|signatory| valid_address(signatory))
		.collect::<ApiResult<BTreeSet<_>>>()
		.map_err(|_| ApiError::bad_request("Signatories must be valid addresses"))?
		.into_iter()
		.collect::<Vec<_>>();
	// pallet-multisig needs at least one other signatory, and a repeated one could never approve
	if signatories.len() != given.len() || signatories.len() < 2 {
		return Err(ApiError::bad_request("A multisig needs at least two different signatories"));
	}
	let threshold = entity["threshold"]
		.as_u64()
		.and_then(|threshold| u16::try_from(threshold).ok())
		.filter(|threshold| (1..=signatories.len() as u16).contains(threshold))
		.ok_or_else(|| {
			ApiError::bad_request("The threshold must be between 1 and the number of signatories")
		})?;
	let address = multisig_address(&signatories, threshold)
		.ok_or_else(|| ApiError::bad_request("Signatories must be valid addresses"))?;
	state
		.db
		.get()
		.await?
		.execute(
			"INSERT INTO multisigs (owner, multisig_address, name, threshold, signatories) VALUES ($1, $2, $3, $4, $5)
			ON CONFLICT (owner, multisig_address) DO UPDATE SET name = $3",
			&[&owner, &address, &name(&entity)?, &(threshold as i16), &Value::from(signatories)],
		)
		.await?;
	Ok(Json(true))
}

pub async fn rename(
	State(state): State<AppState>,
	Path((owner, multisig)): Path<(String, String)>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	let (entity, owner) = owned_entity(&state, &signed, &owner, "update-multisig").await?;
	let multisig = valid_address(&multisig)?;
	state
		.db
		.get()
		.await?
		.execute(
			"UPDATE multisigs SET name = $3 WHERE owner = $1 AND multisig_address = $2",
			&[&owner, &multisig, &name(&entity)?],
		)
		.await?;
	Ok(Json(true))
}

pub async fn remove(
	State(state): State<AppState>,
	Path((owner, multisig)): Path<(String, String)>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	let (_, owner) = owned_entity(&state, &signed, &owner, "remove-multisig").await?;
	let multisig = valid_address(&multisig)?;
	state
		.db
		.get()
		.await?
		.execute(
			"DELETE FROM multisigs WHERE owner = $1 AND multisig_address = $2",
			&[&owner, &multisig],
		)
		.await?;
	Ok(Json(true))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn derives_the_same_account_whatever_the_signatory_order() {
		let (alice, bob) = (
			"15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5".to_owned(),
			"14E5nqKAp3oAJcmzgZhUD2RcptBeUBScxKHgJKU4HPNcKVf3".to_owned(),
		);
		// The address polkadot.js gives for this multisig
		let expected = Some("13yhdvhAzML3u5MENoG8WCT9bTVT5H3C6pjdMpy4PZNisgbS".to_owned());
		assert_eq!(multisig_address(&[alice.clone(), bob.clone()], 2), expected);
		assert_eq!(multisig_address(&[bob, alice], 2), expected);
		assert!(multisig_address(&["not an address".into()], 1).is_none());
	}
}
