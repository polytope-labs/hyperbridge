//! Postgres encodings for the `app` tables SubQuery created.

use std::{error::Error, fmt};

use chrono::{DateTime, NaiveDateTime};
use postgres_types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};
use tokio_postgres::types::private::BytesMut;

type BoxError = Box<dyn Error + Sync + Send>;

/**
 * A Postgres `numeric`, held as its decimal string. SubQuery stores every `BigInt` and `Float`
 * field as `numeric`, and `uint256` values don't fit `rust_decimal`.
 */
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Numeric(String);

impl Numeric {
	/** Parses `-?digits(.digits)?`. */
	pub fn parse(value: &str) -> Result<Self, BoxError> {
		let digits = value.strip_prefix('-').unwrap_or(value);
		let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
		let valid = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
		if int.is_empty() || !valid(int) || !valid(frac) {
			return Err(format!("not a decimal: {value:?}").into());
		}
		Ok(Self(value.to_string()))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl fmt::Display for Numeric {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

/** Anything that prints as an integer: `U256`, `u128`, `i64`, ... */
pub fn numeric(value: impl fmt::Display) -> Numeric {
	let text = value.to_string();
	Numeric::parse(&text).unwrap_or_else(|_| panic!("{text:?} is not a decimal number"))
}

const NUMERIC_POS: u16 = 0x0000;
const NUMERIC_NEG: u16 = 0x4000;

impl ToSql for Numeric {
	/** Postgres' binary numeric: base-10000 digit groups aligned on the decimal point. */
	fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
		let (negative, digits) = match self.0.strip_prefix('-') {
			Some(rest) => (true, rest),
			None => (false, self.0.as_str()),
		};
		let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
		let int = int.trim_start_matches('0');
		let dscale = frac.len() as u16;

		let int_pad = (4 - int.len() % 4) % 4;
		let padded_int = format!("{}{int}", "0".repeat(int_pad));
		let frac_pad = (4 - frac.len() % 4) % 4;
		let padded_frac = format!("{frac}{}", "0".repeat(frac_pad));
		let mut groups: Vec<i16> = padded_int
			.as_bytes()
			.chunks(4)
			.chain(padded_frac.as_bytes().chunks(4))
			.map(|chunk| std::str::from_utf8(chunk).unwrap().parse().unwrap())
			.collect();
		let mut weight = (padded_int.len() / 4) as i16 - 1;
		while groups.first() == Some(&0) {
			groups.remove(0);
			weight -= 1;
		}
		while groups.last() == Some(&0) {
			groups.pop();
		}
		if groups.is_empty() {
			weight = 0;
		}
		let sign = if negative && !groups.is_empty() { NUMERIC_NEG } else { NUMERIC_POS };

		out.extend_from_slice(&(groups.len() as u16).to_be_bytes());
		out.extend_from_slice(&weight.to_be_bytes());
		out.extend_from_slice(&sign.to_be_bytes());
		out.extend_from_slice(&dscale.to_be_bytes());
		for group in groups {
			out.extend_from_slice(&group.to_be_bytes());
		}
		Ok(IsNull::No)
	}

	accepts!(NUMERIC);
	to_sql_checked!();
}

impl<'a> FromSql<'a> for Numeric {
	fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, BoxError> {
		let read = |at: usize| -> Result<u16, BoxError> {
			raw.get(at..at + 2)
				.map(|b| u16::from_be_bytes([b[0], b[1]]))
				.ok_or_else(|| "truncated numeric".into())
		};
		let ndigits = read(0)? as usize;
		let weight = read(2)? as i16 as i32;
		let sign = read(4)?;
		let dscale = read(6)? as usize;
		if sign == 0xC000 {
			return Err("NaN numeric".into());
		}
		let groups = (0..ndigits).map(|i| read(8 + i * 2)).collect::<Result<Vec<_>, _>>()?;

		// Group `i` has weight `weight - i`: weight 0 is units, -1 the first four decimals.
		let group_at = |w: i32| -> u16 {
			let index = weight - w;
			usize::try_from(index).ok().and_then(|i| groups.get(i).copied()).unwrap_or(0)
		};
		let mut int = String::new();
		for w in (0..=weight).rev() {
			if int.is_empty() {
				int = group_at(w).to_string();
			} else {
				int.push_str(&format!("{:04}", group_at(w)));
			}
		}
		if int.is_empty() {
			int.push('0');
		}
		let mut frac = String::new();
		for k in 1..=dscale.div_ceil(4) as i32 {
			frac.push_str(&format!("{:04}", group_at(-k)));
		}
		frac.truncate(dscale);
		let mut text = if frac.is_empty() { int } else { format!("{int}.{frac}") };
		if sign == NUMERIC_NEG {
			text.insert(0, '-');
		}
		Ok(Self(text))
	}

	accepts!(NUMERIC);
}

/** A `timestamp without time zone` from Unix milliseconds, as SubQuery writes `Date` fields. */
pub fn timestamp_ms(ms: u64) -> NaiveDateTime {
	// Event data can carry any value. Out of range clamps rather than panicking the process.
	match DateTime::from_timestamp_millis(i64::try_from(ms).unwrap_or(i64::MAX)) {
		Some(date) => date.naive_utc(),
		None => {
			tracing::warn!(ms, "timestamp out of range; clamped");
			NaiveDateTime::MAX
		},
	}
}

/** A `timestamp without time zone` from Unix seconds. */
pub fn timestamp_secs(secs: u64) -> NaiveDateTime {
	timestamp_ms(secs.saturating_mul(1000))
}

/** Declares a Rust enum for one of SubQuery's Postgres enum types (named by a hash). */
macro_rules! pg_enum {
	($(#[$meta:meta])* $name:ident = $pg:literal { $($variant:ident = $label:literal),+ $(,)? }) => {
		$(#[$meta])*
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ToSql, FromSql)]
		#[postgres(name = $pg)]
		pub enum $name {
			$(#[postgres(name = $label)] $variant),+
		}

		impl $name {
			pub fn as_str(&self) -> &'static str {
				match self { $(Self::$variant => $label),+ }
			}
		}
	};
}

pg_enum!(
	/** `Status`: where a request or response is in its lifecycle. */
	Status = "167d3578e1" {
		Source = "SOURCE",
		HyperbridgeDelivered = "HYPERBRIDGE_DELIVERED",
		Destination = "DESTINATION",
		HyperbridgeTimedOut = "HYPERBRIDGE_TIMED_OUT",
		TimedOut = "TIMED_OUT",
	}
);

pg_enum!(
	OrderStatus = "869e90c211" {
		Placed = "PLACED",
		Filled = "FILLED",
		Redeemed = "REDEEMED",
		Refunded = "REFUNDED",
		Cancelled = "CANCELLED",
	}
);

pg_enum!(
	RelayerRewardType = "1b9d21697d" {
		ConsensusReward = "CONSENSUS_REWARD",
		MessagingReward = "MESSAGING_REWARD",
	}
);

pg_enum!(
	ProtocolParticipantType = "1c84cbf86c" {
		Relayer = "RELAYER",
		Filler = "FILLER",
		User = "USER",
		Referrer = "REFERRER",
	}
);

pg_enum!(
	PointsActivityType = "69ec6d2f99" {
		RewardPointsEarned = "REWARD_POINTS_EARNED",
		OrderPlacedPoints = "ORDER_PLACED_POINTS",
		OrderFilledPoints = "ORDER_FILLED_POINTS",
		TokenTeleportedPoints = "TOKEN_TELEPORTED_POINTS",
		OrderReferredPoints = "ORDER_REFERRED_POINTS",
	}
);

pg_enum!(
	VaultLedgerEventType = "13ba7d8a6a" {
		Deposit = "DEPOSIT",
		Withdraw = "WITHDRAW",
		TransferIn = "TRANSFER_IN",
		TransferOut = "TRANSFER_OUT",
	}
);

pg_enum!(
	TrackedSolverStatus = "709fa19eb8" {
		Pending = "PENDING",
		Tracked = "TRACKED",
	}
);

pg_enum!(
	SolverDiscoveryTrigger = "d75f09d1b9" {
		Fill = "FILL",
		Watchlist = "WATCHLIST",
	}
);

#[cfg(test)]
mod tests {
	use super::*;

	fn round_trip(value: &str) -> String {
		let mut buf = BytesMut::new();
		Numeric::parse(value).unwrap().to_sql(&Type::NUMERIC, &mut buf).unwrap();
		Numeric::from_sql(&Type::NUMERIC, &buf).unwrap().0
	}

	#[test]
	fn numeric_round_trips() {
		for value in [
			"0",
			"1",
			"-1",
			"9999",
			"10000",
			"123456789",
			"115792089237316195423570985008687907853269984665640564039457584007913129639935",
			"0.5",
			"0.0001",
			"0.00001",
			"12.3400",
			"-0.000000000000000001",
			"1000000.000000000000000001",
		] {
			assert_eq!(round_trip(value), value, "{value}");
		}
	}

	#[test]
	fn out_of_range_timestamps_clamp() {
		assert_eq!(timestamp_ms(u64::MAX), NaiveDateTime::MAX);
		assert_eq!(timestamp_secs(u64::MAX), NaiveDateTime::MAX);
		assert_eq!(timestamp_secs(1_700_000_000).to_string(), "2023-11-14 22:13:20");
	}

	#[test]
	fn numeric_rejects_non_decimals() {
		for value in ["", "-", "1e5", "0x10", "1.2.3", ".5"] {
			assert!(Numeric::parse(value).is_err(), "{value}");
		}
	}
}

/** `normalizeTimestamp`: values of 11 digits or fewer are seconds and become milliseconds. */
pub fn normalize_timestamp(timestamp: u64) -> u64 {
	if timestamp.to_string().len() <= 11 {
		timestamp.saturating_mul(1000)
	} else {
		timestamp
	}
}

/** `timestampToDate`: a seconds or milliseconds timestamp as a UTC datetime. */
pub fn timestamp_to_date(timestamp: u64) -> NaiveDateTime {
	timestamp_ms(normalize_timestamp(timestamp))
}

/** `YYYY-MM-DD` of a seconds or milliseconds timestamp, in UTC. */
pub fn utc_date(timestamp: u64) -> String {
	timestamp_to_date(timestamp).format("%Y-%m-%d").to_string()
}
