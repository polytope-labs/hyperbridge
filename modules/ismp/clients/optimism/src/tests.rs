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

use crate::{parse_super_output, Error};
use hex_literal::hex;
use primitive_types::{H256, U256};

/// `extraData` of the super game at `0x69C0Af72663bFDaAd091a6D90bE4De658c0c14DB`, index 86935 of
/// OP Sepolia's dispute game factory. Hashing these bytes gives that game's root claim.
const OP_SEPOLIA_SUPER_OUTPUT: [u8; 73] = hex!(
	"01"
	"000000006ab0ed7c"
	"0000000000000000000000000000000000000000000000000000000000aa37dc"
	"187b295a06cf122f419ea56ffe21f41e772a48ba41d1db682d845ed5778fad46"
);

#[test]
fn decodes_a_live_super_output() {
	let super_output = parse_super_output(&OP_SEPOLIA_SUPER_OUTPUT).unwrap();

	assert_eq!(super_output.timestamp, 1789980028);
	assert_eq!(
		super_output.output_roots.get(&U256::from(11155420u32)),
		Some(&H256(hex!("187b295a06cf122f419ea56ffe21f41e772a48ba41d1db682d845ed5778fad46")))
	);
	assert_eq!(super_output.output_roots.len(), 1);
}

#[test]
fn keeps_every_chain_in_a_multi_chain_set() {
	let mut encoded = OP_SEPOLIA_SUPER_OUTPUT.to_vec();
	encoded.extend_from_slice(&U256::from(8453u32).to_big_endian());
	encoded.extend_from_slice(&[0xab; 32]);

	let super_output = parse_super_output(&encoded).unwrap();

	assert_eq!(super_output.output_roots.len(), 2);
	assert_eq!(super_output.output_roots.get(&U256::from(8453u32)), Some(&H256([0xab; 32])));
}

#[test]
fn rejects_an_unsupported_version() {
	let mut encoded = OP_SEPOLIA_SUPER_OUTPUT;
	encoded[0] = 2;

	assert!(matches!(parse_super_output(&encoded), Err(Error::SuperOutputVersionMismatch(2))));
}

#[test]
fn rejects_a_preimage_with_no_entries() {
	// The header alone decodes to an empty set, which says nothing about any chain.
	assert!(matches!(
		parse_super_output(&OP_SEPOLIA_SUPER_OUTPUT[..9]),
		Err(Error::SuperOutputTooShort(9))
	));
}

#[test]
fn rejects_a_truncated_entry() {
	assert!(matches!(
		parse_super_output(&OP_SEPOLIA_SUPER_OUTPUT[..40]),
		Err(Error::SuperOutputEntriesMalformed(31))
	));
}
