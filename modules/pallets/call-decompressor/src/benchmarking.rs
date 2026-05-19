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

//! Benchmarking for `pallet-call-decompressor`.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use alloc::vec::Vec;
use frame_benchmarking::v2::*;

/// 3 MB of zeros compressed with zstd level 3 (output is 111 bytes). The
/// decompressed length is `CLAIMED_SIZE`. Regenerate with `zstd_safe::compress`
/// if the worst case payload assumption changes.
const COMPRESSED_3MB_ZEROS: [u8; 111] = hex_literal::hex!(
	"28b52ffd8058bfc62d005400001000000100fbff39c002020010000200100002001000020010000200100002001000020010000200100002001000020010000200100002001000020010000200100002001000020010000200100002001000020010000200100002001000fb350e00"
);

const CLAIMED_SIZE: u32 = 2_999_999;

#[benchmarks(
	where
		<T as frame_system::Config>::Hash: From<H256>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		T::Balance: Into<u128>,
		<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp::Call<T>>,
		<T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp_relayer::Call<T>>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn decompress_call() {
		let compressed: Vec<u8> = COMPRESSED_3MB_ZEROS.to_vec();

		#[block]
		{
			let _ = Pallet::<T>::decompress(compressed, CLAIMED_SIZE);
		}
	}
}
