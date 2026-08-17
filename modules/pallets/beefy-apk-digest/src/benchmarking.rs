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

//! Weights for absorbing validator keys into the commitment.
//!
//! The cost is the hashing, and it is linear in the number of slots, so the benchmark sweeps the
//! slot count up to the full circuit width. Reading the relay state proof is not measured: it
//! happens once per block regardless and is the same work the parachain already does to build a
//! block at all.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use apk_commitment::NUM_VALIDATORS;
use frame_benchmarking::v2::*;
use polkadot_sdk::*;

/// The G1 half of a real BEEFY key from a live BLS relay, so the decompression inside
/// `absorb_slots` does the work a live key costs rather than failing early on a made up point.
const A_KEY: [u8; BLS_G1_SIGNATURE_LEN] = hex_literal::hex!(
	"b7235087b611457915f812c4c9af17fe9c590f0a9c9d2f3b62f5d31673a5d5ae02309ea191fe6ebeb66cb3ad23db3b04"
);

#[benchmarks]
mod benches {
	use super::*;

	/// Commit to a full validator set, which is what a block costs when the membership changes.
	#[benchmark]
	fn commit() {
		let keys = alloc::vec![A_KEY; NUM_VALIDATORS];

		#[block]
		{
			super::commit(&keys).expect("the keys are well formed");
		}
	}

	/// Decompressing a full set and nothing else.
	///
	/// A key arrives as 48 compressed bytes and recovering `y` needs a square root in the base
	/// field, which is not obviously cheaper or dearer than the hashing it feeds. Measuring it
	/// apart from `absorb` says which of the two a saving would have to come from.
	#[benchmark]
	fn decompress() {
		let keys = alloc::vec![A_KEY; NUM_VALIDATORS];

		#[block]
		{
			for key in keys.iter() {
				let point = G1Affine::deserialize_compressed(&key[..])
					.expect("the key is a well formed point");
				core::hint::black_box(point);
			}
		}
	}
}
