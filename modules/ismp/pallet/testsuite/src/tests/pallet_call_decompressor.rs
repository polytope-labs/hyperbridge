// Copyright (C) 2023 Polytope Labs.
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

use crate::runtime::{new_test_ext, RuntimeOrigin, Test};
use crate::tests::pallet_ismp_relayer::{encode_accumulate_fees_call, read_file_string};
use ruzstd::StreamingDecoder;
use std::io::Read;
use std::time::Instant;
use zstd_safe::WriteBuf;

#[test]
fn compress_benchmark_with_zstd_safe() {
    new_test_ext().execute_with(|| {
        let start_time = Instant::now();
        let mut buffer = [0u8; 256000];
        let proof_str = read_file_string("src/tests/proofs/accumulate_fee_proof.txt");
        let compressed_proof = zstd_safe::compress(&mut buffer, proof_str.as_bytes(), 3).unwrap();
        let compressed_proof = &buffer[..compressed_proof];
        let end_time = Instant::now();
        let duration = end_time - start_time;
        println!("time taken for compression with zstd_safe {:?}", duration);
        assert!(proof_str.as_bytes().to_vec().len() > compressed_proof.len());

        let start_time = Instant::now();
        let mut buffer = vec![0u8; 25600000000];
        let written = zstd_safe::decompress(&mut buffer[..], compressed_proof).unwrap();
        let decompressed_data = &buffer[..written];
        let end_time = Instant::now();

        let duration = end_time - start_time;
        println!("time taken for decompression with zstd_safe {:?}", duration);

        let start_time = Instant::now();
        let mut decoder = StreamingDecoder::new(compressed_proof.as_slice()).unwrap();
        let mut result = vec![0u8; 2000000];
        let read = decoder.read(&mut result).unwrap();
        dbg!(read / 1000);

        let end_time = Instant::now();
        let duration = end_time - start_time;
        println!("time taken for decompression with ruzstd {:?}", duration);

        dbg!(decompressed_data.len() / 1000);
        assert_eq!(proof_str.as_bytes().to_vec(), decompressed_data);
        assert_eq!(proof_str.as_bytes(), &result[..read]);
    });
}

#[test]
fn decompress_and_execute_call() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        let encoded_call = encode_accumulate_fees_call();

        let mut buffer = vec![0u8; 100000];
        let compressed_call =
            zstd_safe::compress(&mut buffer[..], encoded_call.as_slice(), 3).unwrap();
        let compressed_call = &buffer[..compressed_call];

        pallet_call_decompressor::Pallet::<Test>::decompress_call(
            RuntimeOrigin::none(),
            compressed_call.to_vec(),
            100000,
        )
        .unwrap();
    });
}
