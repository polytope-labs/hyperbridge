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

//! Reconstruction of the exact bytes Arc validators sign for a precommit.
//!
//! Arc signs the SSZ encoding (ethereum_ssz 0.9) of its `Vote` struct with the
//! vote extension stripped:
//!
//! ```text
//! Vote {
//!     typ: VoteType,               // union tag: Prevote = 0, Precommit = 1
//!     height: u64,
//!     round: Option<u32>,          // union: selector byte ++ value
//!     value: Option<H256>,         // union: selector byte ++ block hash
//!     validator_address: [u8; 20],
//! }
//! ```
//!
//! SSZ containers lay out fixed-size fields inline and replace each
//! variable-size field with a 4-byte little-endian offset into a heap appended
//! after the fixed part. `Option<T>` is variable-size, so a precommit for a
//! decided value is always 75 bytes:
//!
//! ```text
//! [0]      0x01                Precommit tag
//! [1..9]   height (u64 LE)
//! [9..13]  offset = 37         round heap position
//! [13..17] offset = 42         value heap position
//! [17..37] validator address
//! [37..42] 0x01 ++ round (u32 LE)
//! [42..75] 0x01 ++ block hash
//! ```
//!
//! There is no chain-id or other domain separation in the signed payload;
//! Arc's `ConsensusSpec` reserves a fork-version field for future signature
//! domain separation but does not mix it into sign-bytes yet.

use alloc::vec::Vec;
use primitive_types::{H160, H256};

/// Byte length of the sign-bytes for a non-nil precommit.
pub const PRECOMMIT_SIGN_BYTES_LEN: usize = 75;

/// Offset of the round union within the SSZ heap.
const ROUND_OFFSET: u32 = 37;

/// Offset of the value union within the SSZ heap.
const VALUE_OFFSET: u32 = ROUND_OFFSET + 5;

/// SSZ union tag for `VoteType::Precommit`.
const PRECOMMIT_TAG: u8 = 1;

/// SSZ union selector for `Option::Some`.
const SOME_SELECTOR: u8 = 1;

/// Build the exact bytes a validator signed for a precommit on `block_hash`
/// at (`height`, `round`).
pub fn precommit_sign_bytes(
	height: u64,
	round: u32,
	block_hash: &H256,
	validator_address: &H160,
) -> Vec<u8> {
	let mut buf = Vec::with_capacity(PRECOMMIT_SIGN_BYTES_LEN);
	buf.push(PRECOMMIT_TAG);
	buf.extend_from_slice(&height.to_le_bytes());
	buf.extend_from_slice(&ROUND_OFFSET.to_le_bytes());
	buf.extend_from_slice(&VALUE_OFFSET.to_le_bytes());
	buf.extend_from_slice(validator_address.as_bytes());
	buf.push(SOME_SELECTOR);
	buf.extend_from_slice(&round.to_le_bytes());
	buf.push(SOME_SELECTOR);
	buf.extend_from_slice(block_hash.as_bytes());
	buf
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloc::vec;

	#[test]
	fn precommit_sign_bytes_layout() {
		let bytes =
			precommit_sign_bytes(1_234_567, 3, &H256::from([0xAA; 32]), &H160::from([0xBB; 20]));
		assert_eq!(bytes.len(), PRECOMMIT_SIGN_BYTES_LEN);
		assert_eq!(bytes[0], 1); // Precommit
		assert_eq!(bytes[1..9], 1_234_567u64.to_le_bytes());
		assert_eq!(bytes[9..13], 37u32.to_le_bytes());
		assert_eq!(bytes[13..17], 42u32.to_le_bytes());
		assert_eq!(bytes[17..37], [0xBB; 20]);
		assert_eq!(bytes[37], 1); // Some(round)
		assert_eq!(bytes[38..42], 3u32.to_le_bytes());
		assert_eq!(bytes[42], 1); // Some(value)
		assert_eq!(bytes[43..75], [0xAA; 32]);
		assert_eq!(
			bytes,
			vec![
				1, 0x87, 0xd6, 0x12, 0, 0, 0, 0, 0, 37, 0, 0, 0, 42, 0, 0, 0, 0xBB, 0xBB, 0xBB,
				0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB,
				0xBB, 0xBB, 0xBB, 1, 3, 0, 0, 0, 1, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
				0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
				0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
			]
		);
	}
}
