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

//! [`sc_simnode::ChainInfo`] implementations for the hyperbridge runtimes
use polkadot_sdk::*;

pub struct GargantuaRuntimeInfo;

impl sc_simnode::ChainInfo for GargantuaRuntimeInfo {
	// make sure you pass the opaque::Block here
	type Block = gargantua_runtime::opaque::Block;
	// the runtime type
	type Runtime = gargantua_runtime::Runtime;
	// the runtime api
	type RuntimeApi = gargantua_runtime::RuntimeApi;
	// [`SignedExtra`] for your runtime
	type SignedExtras = gargantua_runtime::SignedExtra;

	// initialize the [`SignedExtra`] for your runtime, you'll notice I'm calling a pallet method in
	// order to read from storage. This is possible becase this method is called in an externalities
	// provided environment. So feel free to reasd your runtime storage.
	fn signed_extras(
		from: <Self::Runtime as frame_system::pallet::Config>::AccountId,
	) -> Self::SignedExtras {
		use sp_runtime::generic::Era;
		let nonce = frame_system::Pallet::<Self::Runtime>::account_nonce(from);
		(
			frame_system::CheckNonZeroSender::<Self::Runtime>::new(),
			frame_system::CheckSpecVersion::<Self::Runtime>::new(),
			frame_system::CheckTxVersion::<Self::Runtime>::new(),
			frame_system::CheckGenesis::<Self::Runtime>::new(),
			frame_system::CheckEra::<Self::Runtime>::from(Era::Immortal),
			frame_system::CheckNonce::<Self::Runtime>::from(nonce),
			frame_system::CheckWeight::<Self::Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Self::Runtime>::from(0),
			frame_metadata_hash_extension::CheckMetadataHash::new(false),
		)
	}
}
