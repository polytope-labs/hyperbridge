//! USD arithmetic that matches decimal.js as the TypeScript indexer configured it: every
//! `div`, `mul` and `add` rounds to 20 significant digits, half away from zero.

use std::{fmt, num::NonZeroU64, str::FromStr};

use alloy::primitives::U256;
use bigdecimal::{BigDecimal, RoundingMode};

use crate::store::Numeric;

const PRECISION: u64 = 20;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dec(BigDecimal);

impl Dec {
	pub fn zero() -> Self {
		Self(BigDecimal::from(0))
	}

	pub fn one() -> Self {
		Self(BigDecimal::from(1))
	}

	/** `Decimal(value)`: exact, like decimal.js' constructor. */
	pub fn parse(value: &str) -> anyhow::Result<Self> {
		Ok(Self(BigDecimal::from_str(value.trim())?))
	}

	pub fn from_u256(value: U256) -> Self {
		Self(BigDecimal::from_str(&value.to_string()).expect("integers parse"))
	}

	pub fn from_f64(value: f64) -> anyhow::Result<Self> {
		// JavaScript's Number#toString, which is how a number reaches `new Decimal(n)`.
		Self::parse(&format!("{value}"))
	}

	/** `Decimal(10).pow(exponent)`. */
	pub fn pow10(exponent: u32) -> Self {
		Self(BigDecimal::new(1.into(), -(exponent as i64)))
	}

	pub fn div(&self, other: &Dec) -> anyhow::Result<Self> {
		if other.is_zero() {
			anyhow::bail!("division by zero");
		}
		Ok(Self(round(&self.0 / &other.0)))
	}

	pub fn mul(&self, other: &Dec) -> Self {
		Self(round(&self.0 * &other.0))
	}

	pub fn add(&self, other: &Dec) -> Self {
		Self(round(&self.0 + &other.0))
	}

	pub fn sub(&self, other: &Dec) -> Self {
		Self(round(&self.0 - &other.0))
	}

	pub fn is_zero(&self) -> bool {
		self.0 == BigDecimal::from(0)
	}

	pub fn is_positive(&self) -> bool {
		self.0 > BigDecimal::from(0)
	}

	/** `toFixed(places)`: rounded half away from zero, padded to exactly `places` decimals. */
	pub fn to_fixed(&self, places: u32) -> String {
		let rounded = self.0.with_scale_round(places as i64, RoundingMode::HalfUp);
		let text = rounded.to_plain_string();
		// decimal.js writes "-0.00" for a negative value that rounds to zero.
		if self.0 < BigDecimal::from(0) && !text.starts_with('-') {
			format!("-{text}")
		} else {
			text
		}
	}
}

fn round(value: BigDecimal) -> BigDecimal {
	value.with_precision_round(NonZeroU64::new(PRECISION).unwrap(), RoundingMode::HalfUp)
}

impl fmt::Display for Dec {
	/**
	 * decimal.js `toString`: plain notation, no trailing zeros, switching to exponent form when
	 * the exponent is 21 or more, or -7 or less (its `toExpPos` and `toExpNeg` defaults).
	 */
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0.normalized();
		if value == BigDecimal::from(0) {
			return f.write_str("0");
		}
		let (digits, scale) = value.as_bigint_and_exponent();
		let negative = digits.sign() == bigdecimal::num_bigint::Sign::Minus;
		let digits = digits.magnitude().to_string();
		let exponent = digits.len() as i64 - 1 - scale;
		if (-6..21).contains(&exponent) {
			return f.write_str(&value.to_plain_string());
		}
		let sign = if negative { "-" } else { "" };
		let (first, rest) = digits.split_at(1);
		let mantissa = if rest.is_empty() { first.to_string() } else { format!("{first}.{rest}") };
		let exponent_sign = if exponent >= 0 { "+" } else { "" };
		write!(f, "{sign}{mantissa}e{exponent_sign}{exponent}")
	}
}

/**
 * `PriceHelper.getAmountValueInUSD`: `amount / 10^decimals * price`, with the price first fixed
 * to 18 places. Returns `toFixed(18)`.
 */
pub fn amount_value_in_usd(amount: U256, decimals: u8, price: &Dec) -> String {
	let price = Dec::parse(&price.to_fixed(18)).expect("toFixed output parses");
	let units = Dec::from_u256(amount)
		.div(&Dec::pow10(decimals as u32))
		.expect("10^n is non-zero");
	units.mul(&price).to_fixed(18)
}

/** `toScaledUsd`: a USD decimal string as an 18-decimal integer,
 * `Decimal(s).mul(1e18).toFixed(0)`. */
pub fn to_scaled_usd(usd: &str) -> anyhow::Result<Numeric> {
	let scaled = Dec::parse(usd)?.mul(&Dec::pow10(18)).to_fixed(0);
	Numeric::parse(&scaled).map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rounds_to_twenty_significant_digits() {
		let third = Dec::one().div(&Dec::parse("3").unwrap()).unwrap();
		assert_eq!(third.to_string(), "0.33333333333333333333");
		let two_thirds = Dec::parse("2").unwrap().div(&Dec::parse("3").unwrap()).unwrap();
		assert_eq!(two_thirds.to_string(), "0.66666666666666666667");
		let big = Dec::parse("123456789012345678901234").unwrap().mul(&Dec::one());
		assert_eq!(big.to_string(), "1.234567890123456789e+23");
		let sum = Dec::parse("0.30000000000000000004").unwrap().add(&Dec::parse("0.1").unwrap());
		assert_eq!(sum.to_string(), "0.40000000000000000004");
		assert_eq!(Dec::parse("1e21").unwrap().to_string(), "1e+21");
		assert_eq!(Dec::parse("0.0000001").unwrap().to_string(), "1e-7");
		assert_eq!(Dec::parse("0.000001").unwrap().to_string(), "0.000001");
	}

	#[test]
	fn to_fixed_pads_and_rounds_half_up() {
		assert_eq!(Dec::parse("1.5").unwrap().to_fixed(0), "2");
		assert_eq!(Dec::parse("-1.5").unwrap().to_fixed(0), "-2");
		assert_eq!(Dec::parse("0.1").unwrap().to_fixed(3), "0.100");
		assert_eq!(Dec::zero().to_fixed(18), "0.000000000000000000");
		assert_eq!(Dec::parse("-0.001").unwrap().to_fixed(2), "-0.00");
	}

	#[test]
	fn usd_value_of_an_amount() {
		let usdc = amount_value_in_usd(U256::from(25_000_000u64), 6, &Dec::one());
		assert_eq!(usdc, "25.000000000000000000");
		let eth = amount_value_in_usd(
			U256::from(1_234_567_890_123_456_789u128),
			18,
			&Dec::parse("3456.78").unwrap(),
		);
		// Expected values are decimal.js' output for the same inputs.
		assert_eq!(eth, "4267.629591220962959100");
		let big = amount_value_in_usd(
			"123456789012345678901234567890".parse().unwrap(),
			18,
			&Dec::parse("0.123456789").unwrap(),
		);
		assert_eq!(big, "15241578751.714678875000000000");
	}

	#[test]
	fn scaled_usd() {
		assert_eq!(to_scaled_usd("100").unwrap().as_str(), "100000000000000000000");
		assert_eq!(to_scaled_usd("1.5").unwrap().as_str(), "1500000000000000000");
		// 20 significant digits survive the scaling.
		assert_eq!(
			to_scaled_usd("4267.643652735913574012").unwrap().as_str(),
			"4267643652735913574000"
		);
		assert_eq!(to_scaled_usd("0.000000000000000001").unwrap().as_str(), "1");
	}
}
