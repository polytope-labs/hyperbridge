use chrono::{DateTime, SecondsFormat};
use serde::Serialize;
use serde_json::{json, Value};
use sp_core::crypto::{AccountId32, Ss58Codec};

/// What a forum post is attached to. On-chain kinds share their index with the post number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostKind {
	Discussion = 0,
	Referendum = 1,
	Motion = 2,
	TreasurySpend = 3,
}

impl PostKind {
	pub fn from_code(code: u8) -> Option<Self> {
		[Self::Discussion, Self::Referendum, Self::Motion, Self::TreasurySpend]
			.into_iter()
			.find(|kind| *kind as u8 == code)
	}

	/// The post type name the forum frontend uses in `refToPost` and reports.
	pub fn post_type(self) -> &'static str {
		match self {
			Self::Discussion => "post",
			Self::Referendum => "referendaReferendum",
			Self::Motion => "techCommMotion",
			Self::TreasurySpend => "treasurySpend",
		}
	}
}

/// The forum frontend addresses posts by a 24 character hex id. Ours is the kind followed by the
/// post number, so an id can always be resolved back to its post without a lookup table.
pub fn post_id(kind: PostKind, number: i64) -> String {
	format!("{:02x}{number:022x}", kind as u8)
}

pub fn parse_post_id(id: &str) -> Option<(PostKind, i64)> {
	if id.len() != 24 {
		return None;
	}
	let kind = PostKind::from_code(u8::from_str_radix(&id[..2], 16).ok()?)?;
	Some((kind, i64::from_str_radix(&id[2..], 16).ok()?))
}

/// Comment ids take the same shape as post ids, under a prefix no post kind uses, so a reaction's
/// target is never ambiguous.
pub fn comment_id(id: i64) -> String {
	format!("ff{id:022x}")
}

pub fn parse_comment_id(id: &str) -> Option<i64> {
	i64::from_str_radix(id.strip_prefix("ff").filter(|rest| rest.len() == 22)?, 16).ok()
}

/// Milliseconds since the epoch as the ISO string the forum frontend reads, such as
/// `2025-12-05T08:44:18.000Z`.
pub fn iso(millis: i64) -> String {
	DateTime::from_timestamp_millis(millis)
		.map(|time| time.to_rfc3339_opts(SecondsFormat::Millis, true))
		.unwrap_or_default()
}

/// Where a chain event happened.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Indexer {
	pub block_height: i32,
	pub block_hash: String,
	pub block_time: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub extrinsic_index: Option<i32>,
	pub event_index: i32,
}

/// The account behind a post. Wallet users have no username, so one is derived from the key.
pub fn author(address: &str) -> Value {
	let public_key = AccountId32::from_ss58check(address)
		.map(|account| hex::encode(<[u8; 32]>::from(account)))
		.unwrap_or_default();
	json!({
		"username": format!("polkadot-key-0x{public_key}"),
		"publicKey": public_key,
		"address": address,
	})
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
	pub items: Vec<T>,
	pub page: i64,
	pub page_size: i64,
	pub total: i64,
}

/// Paging as the frontend sends it, one based with a page size.
#[derive(Clone, Copy)]
pub struct Paging {
	pub page: i64,
	pub page_size: i64,
}

impl Paging {
	pub fn offset(self) -> i64 {
		(self.page - 1) * self.page_size
	}

	pub fn page<T>(self, items: Vec<T>, total: i64) -> Page<T> {
		Page { items, page: self.page, page_size: self.page_size, total }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn post_ids_round_trip_and_never_clash_with_comment_ids() {
		let id = post_id(PostKind::Referendum, 80);
		assert_eq!(id.len(), 24);
		assert_eq!(parse_post_id(&id), Some((PostKind::Referendum, 80)));
		assert_eq!(parse_comment_id(&comment_id(5)), Some(5));
		assert_eq!(parse_post_id(&comment_id(80)), None);
		assert_eq!(parse_comment_id(&id), None);
	}

	#[test]
	fn formats_times_and_authors_as_the_frontend_reads_them() {
		assert_eq!(iso(1_764_924_258_000), "2025-12-05T08:44:18.000Z");
		let author = author("15PCbSw1gdtCG9fp1AdLVEQnovPh7LLYWN9krTDAZYhHirSY");
		assert!(author["username"]
			.as_str()
			.is_some_and(|name| name.starts_with("polkadot-key-0x")));
		assert_eq!(author["publicKey"].as_str().map(str::len), Some(64));
	}
}
