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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;
pub mod consensus;

pub use beefy_verifier_primitives::{PROOF_TYPE_APK, PROOF_TYPE_NAIVE, PROOF_TYPE_SP1};
pub use consensus::{BEEFY_CONSENSUS_ID, BeefyConsensusClient};

use polkadot_sdk::*;

/// Crypto implementation using substrate host functions
pub struct SubstrateCrypto;

impl ismp::messaging::Keccak256 for SubstrateCrypto {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

impl beefy_verifier::EcdsaRecover for SubstrateCrypto {
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]> {
		sp_io::crypto::secp256k1_ecdsa_recover(signature, prehash)
			.map_err(|_| anyhow::anyhow!("Failed to recover secp256k1 public key"))
	}
}

/// Provides parachain tracking and SP1 vkey data to the BEEFY consensus client.
pub trait BeefyClientConfig {
	/// Returns true if the given parachain id is tracked by this consensus client.
	fn is_parachain_tracked(para_id: u32) -> bool;

	/// Returns the SP1 verification key hash.
	fn sp1_vkey_hash() -> primitive_types::H256;

	/// Verifying key for the aggregate public key circuit, read on every apk proof.
	///
	/// This is the key itself rather than a hash, since the verification happens here. A chain
	/// that never sees an apk proof can leave it empty.
	fn apk_verifying_key() -> alloc::vec::Vec<u8> {
		Default::default()
	}

	/// The parachain whose header digests carry apk commitments, which is hyperbridge.
	///
	/// Every parachain in a proof is proven against the heads root, so a digest from any of them
	/// is authentic; only this one's says anything about the relay chain's authorities. The
	/// default of zero matches no parachain, so a chain that never sees an apk proof learns
	/// nothing rather than learning from whoever turns up.
	fn apk_digest_para_id() -> u32 {
		0
	}

	/// Allowed proof types. Controls which consensus proof formats this client will
	/// accept. On mainnet set to `&[PROOF_TYPE_SP1]`, on testnets set to
	/// `&[PROOF_TYPE_NAIVE, PROOF_TYPE_SP1]`. A proof whose type byte is not listed is
	/// rejected with [`beefy_verifier::error::Error::UnknownProofType`] before verification.
	fn allowed_proof_types() -> &'static [u8];
}
