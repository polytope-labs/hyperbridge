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

//! Proving by running `gnark-apk-proofs` as a separate process.
//!
//! Linking the prover into this binary is the obvious thing and it does not work: the circuit
//! setup wedges inside the relayer's process while the identical call completes in a plain binary,
//! in a tokio runtime, and in a tokio runtime with forty live threads. Rather than keep hunting,
//! the prover runs where it is known to work and the two sides meet over json.
//!
//! It also keeps cgo, a Go toolchain and an 800MB reference string out of this build entirely.

use std::path::PathBuf;

use alloy_primitives::U256;
use anyhow::{anyhow, Context};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField};
use tokio::process::Command;

use crate::{decompress_g1, ApkProof, ApkProofRequest, ApkProver, BITLIST_WORDS};

/// Runs the prover binary, handing it a directory to read inputs from and write the proof to.
///
/// The binary is expected to read `apk-inputs.json` and write `apk-snark.json`, which is what
/// `gnark-apk-proofs`' `prove_from_json` does.
pub struct CommandProver {
	/// The prover executable.
	binary: PathBuf,
	/// Where the two json files live. Each proof overwrites them, so this is not shared.
	work_dir: PathBuf,
}

impl CommandProver {
	/// Build a prover that shells out to `binary`, exchanging files under `work_dir`.
	pub fn new(binary: PathBuf, work_dir: PathBuf) -> Result<Self, anyhow::Error> {
		std::fs::create_dir_all(&work_dir)
			.with_context(|| format!("could not create the prover work directory {work_dir:?}"))?;
		Ok(Self { binary, work_dir })
	}
}

#[async_trait::async_trait]
impl ApkProver for CommandProver {
	async fn prove(&self, request: ApkProofRequest) -> Result<ApkProof, anyhow::Error> {
		// The circuit takes uncompressed coordinates, so the compressed keys are opened up here
		// rather than teaching the prover binary about our encoding.
		let keys = request
			.keys
			.iter()
			.map(|key| {
				let point = decompress_g1(key)?;
				let (x, y) = point.xy().ok_or_else(|| anyhow!("authority key is the identity"))?;
				let mut packed = Vec::with_capacity(96);
				packed.extend_from_slice(&x.into_bigint().to_bytes_be());
				packed.extend_from_slice(&y.into_bigint().to_bytes_be());
				Ok(hex::encode(packed))
			})
			.collect::<Result<Vec<_>, anyhow::Error>>()?;

		let inputs = json::json!({
			"keys": keys,
			"participation": request.participation,
		});
		let input_path = self.work_dir.join("apk-inputs.json");
		let output_path = self.work_dir.join("apk-snark.json");
		tokio::fs::write(&input_path, json::to_vec(&inputs)?)
			.await
			.with_context(|| format!("could not write {input_path:?}"))?;
		// Left over output would be read back as this proof's if the prover failed silently.
		let _ = tokio::fs::remove_file(&output_path).await;

		let status = Command::new(&self.binary)
			.arg(&self.work_dir)
			.status()
			.await
			.with_context(|| format!("could not run the prover at {:?}", self.binary))?;
		if !status.success() {
			Err(anyhow!("the prover exited with {status}"))?
		}

		let output: json::Value = json::from_slice(
			&tokio::fs::read(&output_path)
				.await
				.with_context(|| format!("the prover wrote no proof to {output_path:?}"))?,
		)?;

		let hex_field = |name: &str| -> Result<Vec<u8>, anyhow::Error> {
			let raw = output[name].as_str().ok_or_else(|| anyhow!("proof has no {name}"))?;
			Ok(hex::decode(raw.trim_start_matches("0x"))?)
		};

		let bitlist: [U256; BITLIST_WORDS] = output["bitlist"]
			.as_array()
			.ok_or_else(|| anyhow!("proof has no bitlist"))?
			.iter()
			.map(|word| {
				let raw = word.as_str().ok_or_else(|| anyhow!("bitlist word is not a string"))?;
				Ok(U256::from_be_slice(&hex::decode(raw.trim_start_matches("0x"))?))
			})
			.collect::<Result<Vec<_>, anyhow::Error>>()?
			.try_into()
			.map_err(|_| anyhow!("bitlist is not {BITLIST_WORDS} words"))?;

		let commitment = hex_field("apkCommitment")?;
		let apk_commitment = <[u8; 32]>::try_from(commitment.as_slice())
			.map_err(|_| anyhow!("apk commitment is not 32 bytes"))?;

		Ok(ApkProof { proof: hex_field("apkProof")?, bitlist, apk_commitment })
	}
}
