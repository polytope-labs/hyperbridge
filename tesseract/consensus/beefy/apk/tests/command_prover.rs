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

//! The file contract between this crate and the prover binary.
//!
//! A stub stands in for `gnark-apk-proofs` so these run in a second rather than minutes. What is
//! under test is the handover: what we write, what we read back, and what happens when the prover
//! misbehaves.

use std::{os::unix::fs::PermissionsExt, path::PathBuf};

use apk_beefy::{ApkProofRequest, ApkProver, CommandProver};
use ark_ec::AffineRepr;
use ark_serialize::CanonicalSerialize;

/// A key the prover can decompress, which is all this test needs of it.
fn a_key() -> [u8; 48] {
	let mut compressed = Vec::new();
	ark_bls12_381::G1Affine::generator()
		.serialize_compressed(&mut compressed)
		.unwrap();
	compressed.try_into().unwrap()
}

fn request() -> ApkProofRequest {
	ApkProofRequest { keys: vec![a_key(), a_key()], participation: vec![0, 1] }
}

/// Writes a stub prover into `dir` and returns its path. The body is shell.
fn stub_prover(dir: &PathBuf, body: &str) -> PathBuf {
	let path = dir.join("stub-prover");
	std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
	std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
	path
}

fn work_dir(name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("apk-command-prover-{name}"));
	let _ = std::fs::remove_dir_all(&dir);
	std::fs::create_dir_all(&dir).unwrap();
	dir
}

#[tokio::test]
async fn reads_back_what_the_prover_wrote() {
	let dir = work_dir("happy");
	// Echoes a proof shaped like the real one, and proves the inputs arrived by counting the keys.
	let prover = stub_prover(
		&dir,
		r#"test -f "$1/apk-inputs.json" || exit 3
cat > "$1/apk-snark.json" <<'JSON'
{
  "apkProof": "aabbcc",
  "bitlist": ["03", "00", "00", "00", "00"],
  "apkCommitment": "1111111111111111111111111111111111111111111111111111111111111111"
}
JSON"#,
	);

	let proof = CommandProver::new(prover, dir.clone()).unwrap().prove(request()).await.unwrap();

	assert_eq!(proof.proof, vec![0xaa, 0xbb, 0xcc]);
	assert_eq!(proof.bitlist[0], alloy_primitives::U256::from(3));
	assert_eq!(proof.apk_commitment, [0x11u8; 32]);

	// The keys reached the prover as uncompressed 96 byte points, which is what it expects.
	let written: json::Value =
		json::from_slice(&std::fs::read(dir.join("apk-inputs.json")).unwrap()).unwrap();
	assert_eq!(written["keys"].as_array().unwrap().len(), 2);
	assert_eq!(written["keys"][0].as_str().unwrap().len(), 192);
	assert_eq!(written["participation"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn a_failing_prover_is_an_error() {
	let dir = work_dir("failing");
	let prover = stub_prover(&dir, "exit 1");

	let error = CommandProver::new(prover, dir).unwrap().prove(request()).await.unwrap_err();
	assert!(format!("{error}").contains("exited"), "unexpected error: {error}");
}

/// The dangerous case: a prover that fails without writing, leaving the previous proof behind.
#[tokio::test]
async fn stale_output_is_never_read_back() {
	let dir = work_dir("stale");
	std::fs::write(dir.join("apk-snark.json"), r#"{"apkProof":"dead"}"#).unwrap();
	let prover = stub_prover(&dir, "exit 0");

	let error = CommandProver::new(prover, dir).unwrap().prove(request()).await.unwrap_err();
	assert!(format!("{error}").contains("no proof"), "unexpected error: {error}");
}
