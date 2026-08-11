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

//! Proving in process with `gnark-apk-proofs`.

use std::{path::PathBuf, sync::Arc};

use alloy_primitives::U256;
use anyhow::anyhow;
use ark_serialize_05::CanonicalDeserialize;
use gnark_apk_prover::{G1Affine, ProofBuilder, ProverContext};

use crate::{ApkProof, ApkProofRequest, ApkProver, BITLIST_WORDS};

/// Width of one public input, and of a bitlist word.
const WORD: usize = 32;

/// The public inputs the circuit exposes: the bitlist, then the commitment, then the aggregate key
/// as twelve limbs.
const PUBLIC_INPUTS: usize = 18;

/// Proves with the circuit compiled into this process.
///
/// Construction runs the setup, which compiles the circuit and generates the keys. That takes a
/// few minutes and a large structured reference string, so build one at startup and keep it.
pub struct LocalProver {
	context: Arc<ProverContext>,
}

impl LocalProver {
	/// Compile the circuit and generate the proving key.
	///
	/// `srs_dir` holds the structured reference string. Leave it unset for
	/// `$HOME/.config/gnark-apk-proofs/srs`, which the Go side populates from the Filecoin
	/// ceremony if it is empty.
	pub fn new(srs_dir: Option<PathBuf>) -> Result<Self, anyhow::Error> {
		let context = ProverContext::setup(srs_dir.as_deref())
			.map_err(|e| anyhow!("APK circuit setup failed: {e}"))?;
		Ok(Self { context: Arc::new(context) })
	}
}

#[async_trait::async_trait]
impl ApkProver for LocalProver {
	async fn prove(&self, request: ApkProofRequest) -> Result<ApkProof, anyhow::Error> {
		let keys = request
			.keys
			.iter()
			.map(|key| {
				G1Affine::deserialize_compressed(&key[..])
					.map_err(|_| anyhow!("Malformed G1 authority key"))
			})
			.collect::<Result<Vec<_>, _>>()?;
		let participation = request
			.participation
			.iter()
			.map(|index| {
				u16::try_from(*index)
					.map_err(|_| anyhow!("Authority index {index} is out of range"))
			})
			.collect::<Result<Vec<_>, _>>()?;

		// Proving is minutes of cpu, so keep it off the runtime's worker threads.
		let context = self.context.clone();
		let proof = tokio::task::spawn_blocking(move || {
			ProofBuilder::new(&context)
				.public_keys(keys)
				.participation(participation)
				.prove()
				.map_err(|e| anyhow!("APK proving failed: {e}"))
		})
		.await??;

		let inputs = proof.public_inputs_calldata();
		if inputs.len() != PUBLIC_INPUTS * WORD {
			Err(anyhow!("Expected {PUBLIC_INPUTS} public inputs, got {}", inputs.len() / WORD))?
		}
		let word = |i: usize| &inputs[i * WORD..(i + 1) * WORD];

		let bitlist: [U256; BITLIST_WORDS] = (0..BITLIST_WORDS)
			.map(|i| U256::from_be_slice(word(i)))
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| anyhow!("Bitlist is not {BITLIST_WORDS} words"))?;

		let mut apk_commitment = [0u8; WORD];
		apk_commitment.copy_from_slice(word(BITLIST_WORDS));

		Ok(ApkProof { proof: proof.proof_calldata().to_vec(), bitlist, apk_commitment })
	}
}
