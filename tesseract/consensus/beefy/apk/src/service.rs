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

//! Proving through a prover kept alive between proofs.
//!
//! [`crate::CommandProver`] runs the prover afresh for every proof, and the prover compiles the
//! circuit and generates its keys before it can do anything, about four minutes. Paid once that is
//! setup; paid per proof it is most of the wall clock. This keeps one process alive and talks to it
//! over its stdin and stdout, so the four minutes happen at startup and a proof costs only the two
//! minutes it actually takes.

use std::{path::PathBuf, process::Stdio};

use alloy_primitives::U256;
use anyhow::{anyhow, Context};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField};
use tokio::{
	io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
	process::{Child, ChildStdin, ChildStdout, Command},
	sync::Mutex,
};

use crate::{decompress_g1, ApkProof, ApkProofRequest, ApkProver, BITLIST_WORDS};

/// Read until the prover says something in json.
///
/// It shares stdout with the go library underneath it, which logs its progress there, so anything
/// that is not json is that library talking rather than a reply to us.
async fn read_json_line(stdout: &mut BufReader<ChildStdout>) -> Result<json::Value, anyhow::Error> {
	loop {
		let mut line = String::new();
		if stdout.read_line(&mut line).await? == 0 {
			Err(anyhow!("the prover closed its output"))?
		}
		if let Ok(value) = json::from_str::<json::Value>(line.trim()) {
			return Ok(value);
		}
	}
}

/// A prover process, and the pipes to talk to it.
struct Session {
	/// Kept so the process is killed when this is dropped rather than outliving the relayer.
	_child: Child,
	stdin: ChildStdin,
	stdout: BufReader<ChildStdout>,
}

/// Talks to a prover that stays running between proofs.
///
/// The binary is expected to answer one json request per line with one json response per line,
/// which is what `gnark-apk-proofs`' `prove_serve` does.
pub struct ServiceProver {
	/// One proof at a time. The prover is single threaded and the protocol is a line each way, so
	/// two callers sharing it would read each other's answers.
	session: Mutex<Session>,
}

impl ServiceProver {
	/// Start the prover and wait for it to finish its setup.
	///
	/// This blocks for as long as the circuit takes to compile, so it belongs in startup rather
	/// than on the path of the first proof.
	pub async fn new(binary: PathBuf, srs_dir: Option<PathBuf>) -> Result<Self, anyhow::Error> {
		let mut command = Command::new(&binary);
		if let Some(dir) = srs_dir {
			command.arg(dir);
		}
		let mut child = command
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.with_context(|| format!("could not start the prover at {binary:?}"))?;

		let stdin = child.stdin.take().ok_or_else(|| anyhow!("prover has no stdin"))?;
		let mut stdout =
			BufReader::new(child.stdout.take().ok_or_else(|| anyhow!("prover has no stdout"))?);

		// The prover announces itself once the setup is done, so a proof is never sent into a
		// process that is still compiling and would look like a hang.
		let ready = read_json_line(&mut stdout).await.context("the prover exited during setup")?;
		if ready["ready"].as_bool() != Some(true) {
			Err(anyhow!("the prover failed to start: {ready}"))?
		}

		Ok(Self { session: Mutex::new(Session { _child: child, stdin, stdout }) })
	}
}

#[async_trait::async_trait]
impl ApkProver for ServiceProver {
	async fn prove(&self, request: ApkProofRequest) -> Result<ApkProof, anyhow::Error> {
		// The circuit takes uncompressed coordinates, so the compressed keys are opened up here
		// rather than teaching the prover about our encoding.
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

		let mut line = json::to_string(&json::json!({
			"keys": keys,
			"participation": request.participation,
		}))?;
		line.push('\n');

		let mut session = self.session.lock().await;
		session.stdin.write_all(line.as_bytes()).await.context("the prover is gone")?;
		session.stdin.flush().await.context("the prover is gone")?;

		let response = read_json_line(&mut session.stdout)
			.await
			.context("the prover exited while proving")?;
		if let Some(error) = response["error"].as_str() {
			Err(anyhow!("the prover refused: {error}"))?
		}

		let hex_field = |name: &str| -> Result<Vec<u8>, anyhow::Error> {
			let raw = response[name].as_str().ok_or_else(|| anyhow!("proof has no {name}"))?;
			Ok(hex::decode(raw.trim_start_matches("0x"))?)
		};

		let bitlist: [U256; BITLIST_WORDS] = response["bitlist"]
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

		let apk_commitment = <[u8; 32]>::try_from(hex_field("apkCommitment")?.as_slice())
			.map_err(|_| anyhow!("apk commitment is not 32 bytes"))?;

		Ok(ApkProof { proof: hex_field("apkProof")?, bitlist, apk_commitment })
	}
}
