// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! TRON address conversion utilities.
//!
//! TRON uses base58check encoding for human-readable addresses, similar to Bitcoin.
//! The process is:
//! 1. Start with 21-byte hex address (41 + 20-byte EVM address)
//! 2. Compute double SHA-256 hash
//! 3. Take first 4 bytes as checksum
//! 4. Append checksum to address
//! 5. Encode with base58

use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};

/// Convert a TRON hex address (41-prefixed, 42 chars) to base58 format.
///
/// # Example
/// ```ignore
/// let hex = "4166c2dd969de968b5ec93f4f5c8cbb8c45303c457";
/// let base58 = hex_to_base58(hex)?;
/// assert_eq!(base58, "TKXy9kcmk7jXfXh5oQD1mPh7jwvKs4Kpjr");
/// ```
pub fn hex_to_base58(hex_address: &str) -> anyhow::Result<String> {
	// Remove 0x prefix if present
	let hex_clean = hex_address.strip_prefix("0x").unwrap_or(hex_address);

	// Decode hex to bytes
	let address_bytes =
		hex::decode(hex_clean).with_context(|| format!("Invalid hex address: {}", hex_address))?;

	// TRON addresses should be 21 bytes (41 prefix + 20 bytes)
	if address_bytes.len() != 21 {
		return Err(anyhow!(
			"TRON address must be 21 bytes, got {}. Address: {}",
			address_bytes.len(),
			hex_address
		));
	}

	// Verify it starts with 0x41 (TRON mainnet prefix)
	if address_bytes[0] != 0x41 {
		return Err(anyhow!(
			"TRON address must start with 0x41, got 0x{:02x}. Address: {}",
			address_bytes[0],
			hex_address
		));
	}

	// Compute checksum: first 4 bytes of double SHA-256
	let checksum = compute_checksum(&address_bytes);

	// Combine address + checksum
	let mut full_address = address_bytes.clone();
	full_address.extend_from_slice(&checksum);

	// Encode to base58
	let base58_address = bs58::encode(full_address).into_string();

	Ok(base58_address)
}

/// Convert a TRON base58 address to hex format (41-prefixed).
///
/// # Example
/// ```ignore
/// let base58 = "TKXy9kcmk7jXfXh5oQD1mPh7jwvKs4Kpjr";
/// let hex = base58_to_hex(base58)?;
/// assert_eq!(hex, "4166c2dd969de968b5ec93f4f5c8cbb8c45303c457");
/// ```
pub fn base58_to_hex(base58_address: &str) -> anyhow::Result<String> {
	// Decode base58
	let decoded = bs58::decode(base58_address)
		.into_vec()
		.with_context(|| format!("Invalid base58 address: {}", base58_address))?;

	// Should be 25 bytes (21 address + 4 checksum)
	if decoded.len() != 25 {
		return Err(anyhow!(
			"Decoded base58 address must be 25 bytes, got {}. Address: {}",
			decoded.len(),
			base58_address
		));
	}

	// Split address and checksum
	let address_bytes = &decoded[..21];
	let provided_checksum = &decoded[21..];

	// Verify checksum
	let computed_checksum = compute_checksum(address_bytes);
	if provided_checksum != computed_checksum {
		return Err(anyhow!(
			"Invalid checksum for base58 address: {}. Expected {:?}, got {:?}",
			base58_address,
			computed_checksum,
			provided_checksum
		));
	}

	// Return hex string
	Ok(hex::encode(address_bytes))
}

/// Compute the 4-byte checksum for a TRON address.
///
/// The checksum is the first 4 bytes of `SHA256(SHA256(address))`.
fn compute_checksum(address_bytes: &[u8]) -> [u8; 4] {
	let hash1 = Sha256::digest(address_bytes);
	let hash2 = Sha256::digest(&hash1);
	let mut checksum = [0u8; 4];
	checksum.copy_from_slice(&hash2[..4]);
	checksum
}

/// Check if a string looks like a TRON base58 address.
///
/// This is a heuristic check - it verifies the string starts with 'T' and
/// has a reasonable length (typically 34 characters).
pub fn is_base58_address(address: &str) -> bool {
	address.starts_with('T') && address.len() >= 32 && address.len() <= 36
}

/// Check if a string looks like a TRON hex address.
///
/// Verifies the string is 42 hex characters and starts with "41".
pub fn is_hex_address(address: &str) -> bool {
	let clean = address.strip_prefix("0x").unwrap_or(address);
	clean.len() == 42 && clean.starts_with("41") && clean.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
	use super::*;

	// Test with a known TRON mainnet address (verified conversion)
	// This is the TRON foundation address
	const TEST_HEX: &str = "4166c2dd969de968b5ec93f4f5c8cbb8c45303c457";
	const TEST_BASE58: &str = "TKLZN87oRmkAofzmCMLcnDHfB9ypiorwoc";

	#[test]
	fn test_hex_to_base58() {
		let result = hex_to_base58(TEST_HEX).unwrap();
		assert_eq!(result, TEST_BASE58);
	}

	#[test]
	fn test_base58_to_hex() {
		let result = base58_to_hex(TEST_BASE58).unwrap();
		assert_eq!(result, TEST_HEX);
	}

	#[test]
	fn test_round_trip_hex_to_base58_to_hex() {
		let base58 = hex_to_base58(TEST_HEX).unwrap();
		let hex = base58_to_hex(&base58).unwrap();
		assert_eq!(hex, TEST_HEX);
	}

	#[test]
	fn test_round_trip_base58_to_hex_to_base58() {
		let hex = base58_to_hex(TEST_BASE58).unwrap();
		let base58 = hex_to_base58(&hex).unwrap();
		assert_eq!(base58, TEST_BASE58);
	}

	#[test]
	fn test_hex_with_0x_prefix() {
		let hex_with_prefix = format!("0x{}", TEST_HEX);
		let result = hex_to_base58(&hex_with_prefix).unwrap();
		assert_eq!(result, TEST_BASE58);
	}

	#[test]
	fn test_invalid_hex_length() {
		let result = hex_to_base58("41abcd");
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("21 bytes"));
	}

	#[test]
	fn test_invalid_hex_prefix() {
		// Address starting with 0x00 instead of 0x41
		let result = hex_to_base58("0066c2dd969de968b5ec93f4f5c8cbb8c45303c457");
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("0x41"));
	}

	#[test]
	fn test_invalid_base58_checksum() {
		// Modify the last character to corrupt the checksum
		let mut corrupted = TEST_BASE58.to_string();
		corrupted.pop();
		corrupted.push('Z');

		let result = base58_to_hex(&corrupted);
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("checksum"));
	}

	#[test]
	fn test_is_base58_address() {
		assert!(is_base58_address(TEST_BASE58));
		assert!(is_base58_address("TPL66VK2gCXNCD7EJg9pgJRfqcRazjhUZY"));
		assert!(!is_base58_address(TEST_HEX));
		assert!(!is_base58_address("invalid"));
		assert!(!is_base58_address("T")); // too short
	}

	#[test]
	fn test_is_hex_address() {
		assert!(is_hex_address(TEST_HEX));
		assert!(is_hex_address(&format!("0x{}", TEST_HEX)));
		assert!(!is_hex_address(TEST_BASE58));
		assert!(!is_hex_address("41abcd")); // too short
		assert!(!is_hex_address("0066c2dd969de968b5ec93f4f5c8cbb8c45303c457")); // wrong prefix
	}
}
